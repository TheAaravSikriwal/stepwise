//! Code generation: checked AST → WebAssembly, with trace instrumentation.
//!
//! Spec: `docs/specs/codegen.md`, which lists exactly which trace events
//! each statement emits: that is the contract with the replay engine.
//! Tests: `compiler/tests/codegen.rs` and the golden programs in
//! `compiler/tests/programs/`. See the output with `stepc wat <file>`.
//!
//! The module computes the program's result and, at the same time, calls
//! the trace imports (`abi.rs`) at every step, so the debugger can replay
//! the run afterwards:
//!   - `line` before each statement (and each `if`/`while` condition),
//!   - `declare` / `assign` whenever a variable is made or changed, with the
//!     old value too (read before the new one is computed), so undo is cheap,
//!   - `call` / `ret` around every function, and `scope_exit` whenever a
//!     block ends, including every open block on a `return`.

use crate::abi::{self, Import};
use crate::ast::*;
use crate::checked::{Callee, Checked, ScopeId, Type, VarId};
use crate::span::Span;
use std::collections::HashMap;
use wasm_encoder::{
    BlockType, CodeSection, ExportKind, ExportSection, Function as WasmFunction, FunctionSection,
    ImportSection, Instruction as I, Module, TypeSection, ValType,
};

pub struct Output {
    pub wasm: Vec<u8>,
    /// Indexed by `span_id`: the source range to highlight for each
    /// `line(span_id)` call the module makes. Becomes `DebugTable::steps`.
    pub steps: Vec<Span>,
}

/// Compiles a program that passed the checker. Never panics on a program
/// the checker accepted.
pub fn codegen(program: &Program, checked: &Checked) -> Output {
    // The imports come first (abi.rs): types 0..8 and functions 0..8.
    let mut types = TypeSection::new();
    let mut imports = ImportSection::new();
    abi::emit_imports(&mut types, &mut imports);

    let mut functions = FunctionSection::new();
    let mut code = CodeSection::new();
    let mut exports = ExportSection::new();
    let mut steps = Vec::new();

    for (index, f) in program.functions.iter().enumerate() {
        let sig = &checked.functions[index];
        let results: &[ValType] = if sig.ret == Type::Unit {
            &[]
        } else {
            &[ValType::I32]
        };
        let type_index = types.len();
        types.ty().function(
            vec![ValType::I32; sig.params.len()],
            results.iter().copied(),
        );
        functions.function(type_index);
        if f.name.name == "main" {
            exports.export("main", ExportKind::Func, abi::IMPORT_COUNT + index as u32);
        }
        let body = FnCompiler::compile(index as u32, f, checked, &mut steps);
        code.function(&body);
    }

    let mut module = Module::new();
    module
        .section(&types)
        .section(&imports)
        .section(&functions)
        .section(&exports)
        .section(&code);
    Output {
        wasm: module.finish(),
        steps,
    }
}

/// Compiles one function.
struct FnCompiler<'a> {
    checked: &'a Checked,
    /// Where each of this function's variables lives: parameters are wasm
    /// params 0..n, the rest are locals after them.
    locals: HashMap<VarId, u32>,
    /// A spare local, for holding a return value while the scopes exit.
    scratch: u32,
    /// The blocks currently open, innermost last, for `return`.
    open_scopes: Vec<ScopeId>,
    /// Shared across functions: span ids are numbered in source order.
    steps: &'a mut Vec<Span>,
    func: WasmFunction,
}

impl<'a> FnCompiler<'a> {
    fn compile(
        fn_id: u32,
        f: &Function,
        checked: &'a Checked,
        steps: &'a mut Vec<Span>,
    ) -> WasmFunction {
        // The checker numbers a function's variables consecutively,
        // parameters first, which is exactly wasm's order.
        let mut locals = HashMap::new();
        for (var_id, info) in checked.debug.vars.iter().enumerate() {
            if info.fn_id == fn_id {
                let next = locals.len() as u32;
                locals.insert(var_id as VarId, next);
            }
        }
        let total = locals.len() as u32;
        let params = f.params.len() as u32;
        // Every non-parameter variable, plus the scratch local.
        let func = WasmFunction::new([(total - params + 1, ValType::I32)]);

        let mut c = FnCompiler {
            checked,
            locals,
            scratch: total,
            open_scopes: Vec::new(),
            steps,
            func,
        };

        // Entry: `call(fn_id)`, then a `declare` for each parameter.
        c.emit(I::I32Const(fn_id as i32));
        c.trace(Import::Call);
        for (i, p) in f.params.iter().enumerate() {
            let var = c.var_id(&p.name);
            c.emit(I::I32Const(var as i32));
            c.emit(I::LocalGet(i as u32));
            c.trace(Import::Declare);
        }

        c.block(&f.body);

        // Falling off the end. For a function that returns a value the
        // checker has proved this can't happen, but wasm doesn't know that.
        if checked.functions[fn_id as usize].ret == Type::Unit {
            c.emit(I::I32Const(0));
            c.trace(Import::Ret);
        } else {
            c.emit(I::Unreachable);
        }
        c.emit(I::End);
        c.func
    }

    // ------------------------------------------------------------ helpers

    fn emit(&mut self, instruction: I) {
        self.func.instruction(&instruction);
    }

    fn trace(&mut self, import: Import) {
        self.emit(I::Call(import.func_index()));
    }

    /// Starts a step: records the span to highlight and emits `line`.
    fn step(&mut self, span: Span) {
        let id = self.steps.len() as i32;
        self.steps.push(span);
        self.emit(I::I32Const(id));
        self.trace(Import::Line);
    }

    fn var_id(&self, name: &Ident) -> VarId {
        self.checked.vars[&name.span]
    }

    fn local(&self, name: &Ident) -> u32 {
        self.locals[&self.var_id(name)]
    }

    fn scope_exit(&mut self, scope: ScopeId) {
        self.emit(I::I32Const(scope as i32));
        self.trace(Import::ScopeExit);
    }

    // --------------------------------------------------------- statements

    /// A block's statements, then its `scope_exit` when it ends normally.
    fn block(&mut self, b: &Block) {
        let scope = self.checked.scopes[&b.span];
        self.open_scopes.push(scope);
        for stmt in &b.stmts {
            self.statement(stmt);
        }
        self.open_scopes.pop();
        self.scope_exit(scope);
    }

    fn statement(&mut self, s: &Stmt) {
        match &s.kind {
            // line · value · declare(var, value)
            StmtKind::Let { name, value, .. } => {
                self.step(s.span);
                let (var, local) = (self.var_id(name), self.local(name));
                self.emit(I::I32Const(var as i32));
                self.expr(value);
                self.emit(I::LocalTee(local));
                self.trace(Import::Declare);
            }
            // line · value · assign(var, old, new). The old value is read
            // before the new one is computed; nothing in the new value can
            // change a local, since Stepwise has no closures or references.
            StmtKind::Assign { target, value } => {
                self.step(s.span);
                let (var, local) = (self.var_id(target), self.local(target));
                self.emit(I::I32Const(var as i32));
                self.emit(I::LocalGet(local));
                self.expr(value);
                self.emit(I::LocalTee(local));
                self.trace(Import::Assign);
            }
            // line · value, dropped if it has one.
            StmtKind::Expr(e) => {
                self.step(s.span);
                self.expr(e);
                if self
                    .checked
                    .types
                    .get(&e.span)
                    .is_some_and(|t| *t != Type::Unit)
                {
                    self.emit(I::Drop);
                }
            }
            // line(condition) · condition · the chosen block
            StmtKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.step(cond.span);
                self.expr(cond);
                self.emit(I::If(BlockType::Empty));
                self.block(then_block);
                if let Some(b) = else_block {
                    self.emit(I::Else);
                    self.block(b);
                }
                self.emit(I::End);
            }
            // Every check of the condition is a step, including the last,
            // false one, so you can watch the loop decide to stop.
            //
            //   block            ;; br 1 from inside the loop: exit
            //     loop           ;; br 0: back to the top
            //       line · cond · i32.eqz · br_if 1
            //       body · br 0
            StmtKind::While { cond, body } => {
                self.emit(I::Block(BlockType::Empty));
                self.emit(I::Loop(BlockType::Empty));
                self.step(cond.span);
                self.expr(cond);
                self.emit(I::I32Eqz);
                self.emit(I::BrIf(1));
                self.block(body);
                self.emit(I::Br(0));
                self.emit(I::End);
                self.emit(I::End);
            }
            // line · value · scope_exit for every open block, innermost
            // first · ret(value)
            StmtKind::Return(value) => {
                self.step(s.span);
                if let Some(e) = value {
                    self.expr(e);
                    self.emit(I::LocalSet(self.scratch));
                }
                for scope in self.open_scopes.clone().into_iter().rev() {
                    self.scope_exit(scope);
                }
                match value {
                    Some(_) => {
                        self.emit(I::LocalGet(self.scratch));
                        self.trace(Import::Ret);
                        self.emit(I::LocalGet(self.scratch));
                    }
                    None => {
                        self.emit(I::I32Const(0));
                        self.trace(Import::Ret);
                    }
                }
                self.emit(I::Return);
            }
        }
    }

    // -------------------------------------------------------- expressions

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Int(n) => self.emit(I::I32Const(*n)),
            ExprKind::Bool(b) => self.emit(I::I32Const(i32::from(*b))),
            ExprKind::Var(name) => {
                let local = self.local(name);
                self.emit(I::LocalGet(local));
            }
            ExprKind::Unary { op, operand } => match op {
                UnaryOp::Neg => {
                    self.emit(I::I32Const(0));
                    self.expr(operand);
                    self.emit(I::I32Sub);
                }
                UnaryOp::Not => {
                    self.expr(operand);
                    self.emit(I::I32Eqz);
                }
            },
            // `&&` and `||` must not evaluate their right side when the
            // left already decides: it might call a function that prints.
            ExprKind::Binary {
                op: BinaryOp::And,
                lhs,
                rhs,
            } => {
                self.expr(lhs);
                self.emit(I::If(BlockType::Result(ValType::I32)));
                self.expr(rhs);
                self.emit(I::Else);
                self.emit(I::I32Const(0));
                self.emit(I::End);
            }
            ExprKind::Binary {
                op: BinaryOp::Or,
                lhs,
                rhs,
            } => {
                self.expr(lhs);
                self.emit(I::If(BlockType::Result(ValType::I32)));
                self.emit(I::I32Const(1));
                self.emit(I::Else);
                self.expr(rhs);
                self.emit(I::End);
            }
            ExprKind::Binary { op, lhs, rhs } => {
                self.expr(lhs);
                self.expr(rhs);
                self.emit(match op {
                    BinaryOp::Add => I::I32Add,
                    BinaryOp::Sub => I::I32Sub,
                    BinaryOp::Mul => I::I32Mul,
                    BinaryOp::Div => I::I32DivS,
                    BinaryOp::Rem => I::I32RemS,
                    BinaryOp::Eq => I::I32Eq,
                    BinaryOp::Ne => I::I32Ne,
                    BinaryOp::Lt => I::I32LtS,
                    BinaryOp::Le => I::I32LeS,
                    BinaryOp::Gt => I::I32GtS,
                    BinaryOp::Ge => I::I32GeS,
                    BinaryOp::And | BinaryOp::Or => unreachable!("handled above"),
                });
            }
            ExprKind::Call { callee, args } => {
                for arg in args {
                    self.expr(arg);
                }
                match self.checked.calls[&callee.span] {
                    Callee::Function(id) => self.emit(I::Call(abi::IMPORT_COUNT + id)),
                    Callee::Print(Type::Bool) => self.trace(Import::PrintBool),
                    Callee::Print(_) => self.trace(Import::Print),
                }
            }
        }
    }
}
