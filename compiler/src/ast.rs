//! The abstract syntax tree: what the parser produces.
//!
//! **Shared contract (draft).** Your parser builds this; the resolver, type
//! checker and codegen read it. Change anything you like, but tell me so the
//! specs and tests stay in sync. Grammar: `docs/specs/parser.md`.
//!
//! Every node carries a [`Span`]. Rules of thumb:
//! - an expression's span covers all of it (`a + b * c` spans `a` to `c`)
//! - a statement's span covers it including the trailing `;`
//! - a name's span covers just the name, for "defined here" labels

use crate::span::Span;

/// A whole program: a list of functions. Execution starts at `main`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: Ident,
    pub params: Vec<Param>,
    /// `None` when there's no `-> type`.
    pub return_type: Option<TypeExpr>,
    pub body: Block,
    /// From `fn` to the closing `}`.
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeExpr,
}

/// A name as written in the source, e.g. a variable, function or type name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

/// A type as written, e.g. `int`. Just a name for now; arrays come in v1.0.
/// The resolver decides whether the name is a real type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeExpr {
    pub name: Ident,
}

/// `{ statements }`. Blocks introduce a scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    /// From `{` to `}`.
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StmtKind {
    /// `let x = e;` or `let mut x = e;`, with an optional `: type`.
    Let {
        mutable: bool,
        name: Ident,
        ty: Option<TypeExpr>,
        value: Expr,
    },
    /// `x = e;`
    Assign { target: Ident, value: Expr },
    /// `if cond { ... }`, optionally followed by `else { ... }` or `else if ...`.
    If {
        cond: Expr,
        then_block: Block,
        /// `else if` is represented as an `else` block holding one `If` statement.
        else_block: Option<Block>,
    },
    /// `while cond { ... }`
    While { cond: Expr, body: Block },
    /// `return;` or `return e;`
    Return(Option<Expr>),
    /// An expression followed by `;`, e.g. `print(x);`
    Expr(Expr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    /// An integer literal. The parser folds a `-` directly in front of a
    /// literal into it (`-5` is `Int(-5)`, spanning `-5`), which is how
    /// `-2147483648` fits: every `Int` is a valid `int`.
    Int(i32),
    Bool(bool),
    /// A variable reference.
    Var(Ident),
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    /// `f(a, b)`, including the built-in `print(x)`.
    Call {
        callee: Ident,
        args: Vec<Expr>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    /// `-e`
    Neg,
    /// `!e`
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

// ------------------------------------------------------------------ printing
//
// A compact S-expression form of the tree, without spans. Used by the parser
// tests (so expected trees are one readable line) and by `stepc ast`.
//
//   fn        (fn name ((a int) (b bool)) -> int {...})
//   block     {stmt stmt ...}
//   let       (let x 1)  (let mut x 1)  (let x: int 1)
//   assign    (= x e)
//   if        (if cond {...})  (if cond {...} {...})
//   while     (while cond {...})
//   return    (return)  (return e)
//   expr stmt e
//   exprs     5  -5  true  x  (- e)  (! e)  (+ a b)  (call f a b)

impl Program {
    /// One function per line.
    pub fn to_sexpr(&self) -> String {
        let fns: Vec<String> = self.functions.iter().map(Function::to_sexpr).collect();
        fns.join("\n")
    }
}

impl Function {
    pub fn to_sexpr(&self) -> String {
        let params: Vec<String> = self
            .params
            .iter()
            .map(|p| format!("({} {})", p.name.name, p.ty.name.name))
            .collect();
        let ret = match &self.return_type {
            Some(t) => format!(" -> {}", t.name.name),
            None => String::new(),
        };
        format!(
            "(fn {} ({}){ret} {})",
            self.name.name,
            params.join(" "),
            self.body.to_sexpr()
        )
    }
}

impl Block {
    pub fn to_sexpr(&self) -> String {
        let stmts: Vec<String> = self.stmts.iter().map(Stmt::to_sexpr).collect();
        format!("{{{}}}", stmts.join(" "))
    }
}

impl Stmt {
    pub fn to_sexpr(&self) -> String {
        match &self.kind {
            StmtKind::Let {
                mutable,
                name,
                ty,
                value,
            } => {
                let m = if *mutable { "mut " } else { "" };
                let t = ty
                    .as_ref()
                    .map_or(String::new(), |t| format!(": {}", t.name.name));
                format!("(let {m}{}{t} {})", name.name, value.to_sexpr())
            }
            StmtKind::Assign { target, value } => {
                format!("(= {} {})", target.name, value.to_sexpr())
            }
            StmtKind::If {
                cond,
                then_block,
                else_block,
            } => match else_block {
                Some(e) => format!(
                    "(if {} {} {})",
                    cond.to_sexpr(),
                    then_block.to_sexpr(),
                    e.to_sexpr()
                ),
                None => format!("(if {} {})", cond.to_sexpr(), then_block.to_sexpr()),
            },
            StmtKind::While { cond, body } => {
                format!("(while {} {})", cond.to_sexpr(), body.to_sexpr())
            }
            StmtKind::Return(None) => "(return)".into(),
            StmtKind::Return(Some(e)) => format!("(return {})", e.to_sexpr()),
            StmtKind::Expr(e) => e.to_sexpr(),
        }
    }
}

impl Expr {
    pub fn to_sexpr(&self) -> String {
        match &self.kind {
            ExprKind::Int(n) => n.to_string(),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Var(id) => id.name.clone(),
            ExprKind::Unary { op, operand } => {
                let sym = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                };
                format!("({sym} {})", operand.to_sexpr())
            }
            ExprKind::Binary { op, lhs, rhs } => {
                format!("({} {} {})", op.symbol(), lhs.to_sexpr(), rhs.to_sexpr())
            }
            ExprKind::Call { callee, args } => {
                let mut s = format!("(call {}", callee.name);
                for a in args {
                    s.push(' ');
                    s.push_str(&a.to_sexpr());
                }
                s.push(')');
                s
            }
        }
    }
}

impl BinaryOp {
    /// The operator as written, for error messages.
    pub fn symbol(self) -> &'static str {
        match self {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Eq => "==",
            BinaryOp::Ne => "!=",
            BinaryOp::Lt => "<",
            BinaryOp::Le => "<=",
            BinaryOp::Gt => ">",
            BinaryOp::Ge => ">=",
            BinaryOp::And => "&&",
            BinaryOp::Or => "||",
        }
    }
}
