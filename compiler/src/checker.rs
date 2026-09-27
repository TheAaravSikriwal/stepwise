//! Checker: name resolution and type checking.
//!
//! Spec: `docs/specs/checker.md` (the 19 rules). Tests:
//! `compiler/tests/checker.rs` and the message snapshots in
//! `compiler/tests/error_messages.rs`. Output types: `checked.rs`.
//!
//! Two passes over the program:
//!  1. every function's signature, so calls can reach functions defined
//!     later in the file, and functions can call themselves;
//!  2. every function's body: names are looked up on a stack of scopes, and
//!     every expression is given a type.
//!
//! An expression with an error gets no type (`None`), which is silently
//! compatible with everything afterwards. That's what makes one mistake
//! produce one error, instead of a cascade: an unknown `y` in `let x = y + 1;`
//! is reported once, and every later use of `x` stays quiet.

use crate::ast::*;
use crate::checked::*;
use crate::debug_table::{FunctionInfo, ScopeInfo, TypeName, VarInfo};
use crate::diagnostic::Diagnostic;
use crate::span::Span;
use std::collections::HashMap;

/// A type, or `None` for "already reported an error here".
type Ty = Option<Type>;

/// Checks a program that parsed without errors.
///
/// Reports every problem it can find, never just the first. Never panics.
pub fn check(program: &Program) -> (Checked, Vec<Diagnostic>) {
    let mut c = Checker::default();
    c.signatures(program);
    c.check_main(program);
    for (i, f) in program.functions.iter().enumerate() {
        c.body(i, f);
    }
    (c.out, c.diagnostics)
}

/// A variable the checker knows about.
struct Declared {
    id: VarId,
    ty: Ty,
    mutable: bool,
    is_param: bool,
    /// The name where it was declared, for "made here" labels.
    span: Span,
}

#[derive(Default)]
struct Checker {
    out: Checked,
    diagnostics: Vec<Diagnostic>,
    functions_by_name: HashMap<String, FnId>,
    /// Parameter and return types per function, `None` where unknown.
    signatures: Vec<(Vec<Ty>, Ty)>,
    /// Every variable declared so far, indexed by the scopes below.
    declared: Vec<Declared>,
    /// Innermost last: each scope's id and its names → index into `declared`.
    scopes: Vec<(ScopeId, HashMap<String, usize>)>,
    current_fn: FnId,
    /// The current function's return type (`Unit` if it has none).
    current_ret: Ty,
}

fn article(ty: Type) -> String {
    match ty {
        Type::Int => "an `int`".into(),
        Type::Bool => "a `bool`".into(),
        Type::Unit => "nothing".into(),
    }
}

impl Checker {
    fn error(&mut self, message: impl Into<String>, span: Span) {
        self.diagnostics.push(Diagnostic::error(message, span));
    }

    // ------------------------------------------------------- pass one

    fn signatures(&mut self, program: &Program) {
        for f in &program.functions {
            let params: Vec<Ty> = f.params.iter().map(|p| self.resolve_type(&p.ty)).collect();
            let ret = match &f.return_type {
                None => Some(Type::Unit),
                Some(t) => self.resolve_type(t),
            };
            let id = self.out.functions.len() as FnId;

            if f.name.name == "print" {
                self.diagnostics.push(
                    Diagnostic::error(
                        "`print` is built in, so a function can't be called `print`",
                        f.name.span,
                    )
                    .with_note("give your function a different name"),
                );
            } else if let Some(&first) = self.functions_by_name.get(&f.name.name) {
                let first_span = program.functions[first as usize].name.span;
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("there's already a function named `{}`", f.name.name),
                        f.name.span,
                    )
                    .with_label(first_span, "first made here")
                    .with_note("give one of them a different name"),
                );
            } else {
                self.functions_by_name.insert(f.name.name.clone(), id);
            }

            for (i, p) in f.params.iter().enumerate() {
                if f.params[..i].iter().any(|q| q.name.name == p.name.name) {
                    self.diagnostics.push(
                        Diagnostic::error(
                            format!("two parameters are both called `{}`", p.name.name),
                            p.name.span,
                        )
                        .with_note("each parameter needs its own name"),
                    );
                }
            }

            self.out.functions.push(FnSig {
                name: f.name.name.clone(),
                params: params.iter().map(|t| t.unwrap_or(Type::Int)).collect(),
                ret: ret.unwrap_or(Type::Unit),
            });
            self.out.debug.functions.push(FunctionInfo {
                name: f.name.name.clone(),
                span: f.name.span,
            });
            self.signatures.push((params, ret));
        }
    }

    fn resolve_type(&mut self, t: &TypeExpr) -> Ty {
        match t.name.name.as_str() {
            "int" => Some(Type::Int),
            "bool" => Some(Type::Bool),
            other => {
                self.diagnostics.push(
                    Diagnostic::error(format!("there's no type called `{other}`"), t.name.span)
                        .with_note(
                            "the types are `int` (whole numbers) and `bool` (true or false)",
                        ),
                );
                None
            }
        }
    }

    fn check_main(&mut self, program: &Program) {
        let Some(&id) = self.functions_by_name.get("main") else {
            self.diagnostics.push(
                Diagnostic::error(
                    "this program has no `fn main()` to start from",
                    Span::new(0, 0),
                )
                .with_note("add `fn main() { ... }`: it's where every program starts"),
            );
            return;
        };
        let main = &program.functions[id as usize];
        if !main.params.is_empty() {
            self.diagnostics.push(
                Diagnostic::error("`main` can't take parameters", main.name.span)
                    .with_note("nothing calls `main`, so there's nothing to pass it"),
            );
        }
        if let Some(t) = &main.return_type {
            self.diagnostics.push(
                Diagnostic::error("`main` can't give back a value", t.name.span)
                    .with_note("remove the `->` and the type after it"),
            );
        }
    }

    // ------------------------------------------------------- pass two

    fn body(&mut self, index: usize, f: &Function) {
        self.current_fn = index as FnId;
        self.current_ret = self.signatures[index].1;

        // Parameters live in the function body's scope.
        self.push_scope(f.body.span);
        for (i, p) in f.params.iter().enumerate() {
            let duplicate = f.params[..i].iter().any(|q| q.name.name == p.name.name);
            if !duplicate {
                let ty = self.signatures[index].0[i];
                self.declare(&p.name, ty, false, true);
            }
        }
        self.statements(&f.body.stmts);
        self.scopes.pop();

        if let Some(ret) = self.current_ret
            && ret != Type::Unit
            && !always_returns(&f.body.stmts)
        {
            self.diagnostics.push(
                Diagnostic::error(
                    format!("`{}` doesn't give back a value on every path", f.name.name),
                    f.name.span,
                )
                .with_note("add a `return` at the end, for when none of the other `return`s run"),
            );
        }
    }

    // ------------------------------------------------------- scopes

    fn push_scope(&mut self, span: Span) {
        let id = self.out.debug.scopes.len() as ScopeId;
        let parent = self.scopes.last().map(|(id, _)| *id);
        self.out.debug.scopes.push(ScopeInfo {
            fn_id: self.current_fn,
            parent,
            span,
        });
        self.out.scopes.insert(span, id);
        self.scopes.push((id, HashMap::new()));
    }

    fn lookup(&self, name: &str) -> Option<usize> {
        self.scopes
            .iter()
            .rev()
            .find_map(|(_, names)| names.get(name).copied())
    }

    /// Declares a variable in the innermost scope. Stepwise has no
    /// shadowing: two live variables with the same name would be confusing
    /// in the variables panel, so a name that's already visible is an error.
    fn declare(&mut self, name: &Ident, ty: Ty, mutable: bool, is_param: bool) {
        if let Some(existing) = self.lookup(&name.name) {
            let first = self.declared[existing].span;
            self.diagnostics.push(
                Diagnostic::error(
                    format!("there's already a variable named `{}` here", name.name),
                    name.span,
                )
                .with_label(first, "first made here")
                .with_note(
                    "pick a different name: two variables can't share a name while both exist",
                ),
            );
            return;
        }
        let id = self.out.debug.vars.len() as VarId;
        let (scope_id, _) = self
            .scopes
            .last()
            .expect("declare is only called inside a scope");
        self.out.debug.vars.push(VarInfo {
            name: name.name.clone(),
            ty: ty.and_then(Type::type_name).unwrap_or(TypeName::Int),
            fn_id: self.current_fn,
            scope_id: *scope_id,
            span: name.span,
        });
        self.declared.push(Declared {
            id,
            ty,
            mutable,
            is_param,
            span: name.span,
        });
        let index = self.declared.len() - 1;
        if let Some((_, names)) = self.scopes.last_mut() {
            names.insert(name.name.clone(), index);
        }
        self.out.vars.insert(name.span, id);
    }

    /// Reports a name that isn't a visible variable, as helpfully as possible.
    fn unknown_variable(&mut self, id: &Ident) {
        if self.functions_by_name.contains_key(&id.name) || id.name == "print" {
            self.diagnostics.push(
                Diagnostic::error(
                    format!("`{}` is a function, not a variable", id.name),
                    id.span,
                )
                .with_note(format!("to call it, write `{}()`", id.name)),
            );
            return;
        }
        let mut d = Diagnostic::error(format!("there's no variable named `{}`", id.name), id.span);
        let visible = self
            .scopes
            .iter()
            .flat_map(|(_, names)| names.keys().cloned());
        if let Some(close) = closest(&id.name, visible) {
            d = d.with_note(format!("did you mean `{close}`?"));
        } else {
            d = d.with_note("a variable has to be made with `let` before it's used");
        }
        self.diagnostics.push(d);
    }

    // ------------------------------------------------------- statements

    fn block(&mut self, b: &Block) {
        self.push_scope(b.span);
        self.statements(&b.stmts);
        self.scopes.pop();
    }

    fn statements(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.statement(s);
        }
    }

    fn statement(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                // The value is checked before the name exists, so
                // `let x = x + 1;` can't refer to the new `x`.
                let value_ty = self.value(value);
                let ty = match ty {
                    Some(t) => {
                        let declared = self.resolve_type(t);
                        if let Some(want) = declared {
                            self.expect_type(value, value_ty, want);
                        }
                        declared
                    }
                    None => value_ty,
                };
                self.declare(name, ty, *mutable, false);
            }
            StmtKind::Assign { target, value } => self.assignment(target, value),
            StmtKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.condition(cond);
                self.block(then_block);
                if let Some(b) = else_block {
                    self.block(b);
                }
            }
            StmtKind::While { cond, body } => {
                self.condition(cond);
                self.block(body);
            }
            StmtKind::Return(value) => self.return_statement(s.span, value.as_ref()),
            StmtKind::Expr(e) => {
                self.expr(e);
            }
        }
    }

    fn assignment(&mut self, target: &Ident, value: &Expr) {
        let value_ty = self.value(value);
        let Some(index) = self.lookup(&target.name) else {
            self.unknown_variable(target);
            return;
        };
        let (id, ty, mutable, is_param, made) = {
            let d = &self.declared[index];
            (d.id, d.ty, d.mutable, d.is_param, d.span)
        };
        self.out.vars.insert(target.span, id);
        if !mutable {
            let note = if is_param {
                format!(
                    "parameters can't be changed. Copy it into a new variable first: `let mut new_{0} = {0};`",
                    target.name
                )
            } else {
                format!("to allow changes, make it with `let mut {}`", target.name)
            };
            self.diagnostics.push(
                Diagnostic::error(
                    format!(
                        "`{}` can't be changed, because it wasn't made with `mut`",
                        target.name
                    ),
                    target.span,
                )
                .with_label(made, "made here")
                .with_note(note),
            );
        }
        if let Some(want) = ty {
            self.expect_type(value, value_ty, want);
        }
    }

    fn return_statement(&mut self, span: Span, value: Option<&Expr>) {
        match (self.current_ret, value) {
            (Some(Type::Unit), Some(e)) => {
                self.expr(e);
                self.diagnostics.push(
                    Diagnostic::error("this function doesn't give back a value, so `return` can't have one", e.span)
                        .with_note("to give back a value, add `-> int` (or `-> bool`) after the function's parameters"),
                );
            }
            (Some(Type::Unit), None) | (None, None) => {}
            (Some(want), None) => {
                self.diagnostics.push(
                    Diagnostic::error(
                        format!("this function has to give back {}", article(want)),
                        span,
                    )
                    .with_note("write the value after `return`, like `return 0;`"),
                );
            }
            (Some(want), Some(e)) => {
                let got = self.value(e);
                self.expect_type(e, got, want);
            }
            (None, Some(e)) => {
                self.expr(e);
            }
        }
    }

    /// `if` and `while` conditions must be `bool`.
    fn condition(&mut self, cond: &Expr) {
        if self.value(cond) == Some(Type::Int) {
            let name = match &cond.kind {
                ExprKind::Var(id) => id.name.as_str(),
                _ => "x",
            };
            self.diagnostics.push(
                Diagnostic::error(
                    "a condition has to be `true` or `false`, but this is an `int`",
                    cond.span,
                )
                .with_note(format!("if you meant \"is not zero\", write `{name} != 0`")),
            );
        }
    }

    // ------------------------------------------------------- expressions

    /// Reports `e` if it isn't `want` (and isn't already an error).
    fn expect_type(&mut self, e: &Expr, got: Ty, want: Type) {
        if let Some(got) = got
            && got != want
        {
            self.error(
                format!(
                    "expected {} here, but this is {}",
                    article(want),
                    article(got)
                ),
                e.span,
            );
        }
    }

    /// An expression used as a value: a call to a function that gives
    /// nothing back can't be one.
    fn value(&mut self, e: &Expr) -> Ty {
        let ty = self.expr(e);
        if ty == Some(Type::Unit) {
            let what = match &e.kind {
                ExprKind::Call { callee, .. } => format!("`{}`", callee.name),
                _ => "this".into(),
            };
            self.diagnostics.push(
                Diagnostic::error(
                    format!("{what} doesn't give back a value, so it can't be used here"),
                    e.span,
                )
                .with_note("only functions with `->` after their parameters give back a value"),
            );
            return None;
        }
        ty
    }

    /// Types an expression and records the type for codegen.
    fn expr(&mut self, e: &Expr) -> Ty {
        let ty = self.expr_type(e);
        if let Some(t) = ty {
            self.out.types.insert(e.span, t);
        }
        ty
    }

    fn expr_type(&mut self, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Int(_) => Some(Type::Int),
            ExprKind::Bool(_) => Some(Type::Bool),
            ExprKind::Var(id) => match self.lookup(&id.name) {
                Some(index) => {
                    let d = &self.declared[index];
                    self.out.vars.insert(id.span, d.id);
                    d.ty
                }
                None => {
                    self.unknown_variable(id);
                    None
                }
            },
            ExprKind::Unary { op, operand } => {
                let want = match op {
                    UnaryOp::Neg => Type::Int,
                    UnaryOp::Not => Type::Bool,
                };
                let got = self.value(operand);
                self.expect_type(operand, got, want);
                Some(want)
            }
            ExprKind::Binary { op, lhs, rhs } => self.binary(*op, lhs, rhs),
            ExprKind::Call { callee, args } => self.call(e, callee, args),
        }
    }

    /// The operator table from the spec. The result type is known whatever
    /// the operands were, so a bad operand doesn't spread.
    fn binary(&mut self, op: BinaryOp, lhs: &Expr, rhs: &Expr) -> Ty {
        use BinaryOp::*;
        let l = self.value(lhs);
        let r = self.value(rhs);
        let (operands, result) = match op {
            Add | Sub | Mul | Div | Rem => (Some(Type::Int), Type::Int),
            Lt | Le | Gt | Ge => (Some(Type::Int), Type::Bool),
            And | Or => (Some(Type::Bool), Type::Bool),
            Eq | Ne => (None, Type::Bool),
        };
        match operands {
            Some(want) => {
                for (side, got) in [(lhs, l), (rhs, r)] {
                    if let Some(got) = got
                        && got != want
                    {
                        let kind = if want == Type::Int {
                            "numbers (`int`s)"
                        } else {
                            "`true` and `false` (`bool`s)"
                        };
                        self.error(
                            format!(
                                "`{}` works with {kind}, but this is {}",
                                op.symbol(),
                                article(got)
                            ),
                            side.span,
                        );
                    }
                }
            }
            None => {
                if let (Some(a), Some(b)) = (l, r)
                    && a != b
                {
                    self.error(
                        format!("can't compare {} with {}", article(a), article(b)),
                        rhs.span,
                    );
                }
            }
        }
        Some(result)
    }

    fn call(&mut self, e: &Expr, callee: &Ident, args: &[Expr]) -> Ty {
        if callee.name == "print" {
            let types: Vec<Ty> = args.iter().map(|a| self.value(a)).collect();
            if args.len() == 1 {
                self.out
                    .calls
                    .insert(callee.span, Callee::Print(types[0].unwrap_or(Type::Int)));
            } else {
                self.diagnostics.push(
                    Diagnostic::error("`print` takes exactly one value", e.span)
                        .with_note("to print two things, use two `print`s"),
                );
            }
            return Some(Type::Unit);
        }
        if self.lookup(&callee.name).is_some() {
            for a in args {
                self.value(a);
            }
            self.error(
                format!("`{}` is a variable, not a function", callee.name),
                callee.span,
            );
            return None;
        }
        let Some(&id) = self.functions_by_name.get(&callee.name) else {
            for a in args {
                self.value(a);
            }
            let mut d = Diagnostic::error(
                format!("there's no function named `{}`", callee.name),
                callee.span,
            );
            if let Some(close) = closest(&callee.name, self.functions_by_name.keys().cloned()) {
                d = d.with_note(format!("did you mean `{close}`?"));
            }
            self.diagnostics.push(d);
            return None;
        };

        let (params, ret) = self.signatures[id as usize].clone();
        let types: Vec<Ty> = args.iter().map(|a| self.value(a)).collect();
        if args.len() != params.len() {
            let n = params.len();
            self.error(
                format!(
                    "`{}` needs {n} {}, but was given {}",
                    callee.name,
                    if n == 1 { "value" } else { "values" },
                    args.len()
                ),
                e.span,
            );
        } else {
            for ((arg, got), want) in args.iter().zip(types).zip(params) {
                if let Some(want) = want {
                    self.expect_type(arg, got, want);
                }
            }
        }
        self.out.calls.insert(callee.span, Callee::Function(id));
        ret
    }
}

/// Whether a list of statements is sure to reach a `return`. A `while`
/// never counts (it might run zero times); an `if` counts only when both
/// branches return.
fn always_returns(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::If {
            then_block,
            else_block: Some(else_block),
            ..
        } => always_returns(&then_block.stmts) && always_returns(&else_block.stmts),
        _ => false,
    })
}

/// The candidate closest to `name`, if it's a plausible typo (edit distance
/// 1 or 2, and the name isn't too short for that to mean anything).
fn closest(name: &str, candidates: impl Iterator<Item = String>) -> Option<String> {
    if name.chars().count() <= 2 {
        return None;
    }
    candidates
        .map(|c| (edit_distance(name, &c), c))
        .filter(|(d, _)| *d <= 2)
        .min()
        .map(|(_, c)| c)
}

/// Levenshtein distance, one row at a time.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for j in 0..b.len() {
            let above = row[j + 1];
            row[j + 1] = (diagonal + usize::from(ca != b[j]))
                .min(row[j] + 1)
                .min(above + 1);
            diagonal = above;
        }
    }
    row[b.len()]
}
