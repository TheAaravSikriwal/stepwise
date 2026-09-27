//! Phase 0 placeholder: ignores the source and returns a module equivalent to
//!
//! ```text
//! fn main() {
//!     print(42);
//! }
//! ```
//!
//! including the trace calls real codegen will emit. Delete this file once
//! codegen (Phase 3) produces real modules.

use crate::CompileOutput;
use crate::abi::{self, Import};
use crate::debug_table::{DebugTable, FunctionInfo, ScopeInfo};
use crate::span::Span;
use wasm_encoder::{
    CodeSection, ExportKind, ExportSection, Function, FunctionSection, ImportSection, Instruction,
    Module, TypeSection,
};

pub(crate) fn compile(_source: &str) -> CompileOutput {
    let mut types = TypeSection::new();
    let mut imports = ImportSection::new();
    abi::emit_imports(&mut types, &mut imports);

    let main_type = types.len();
    types.ty().function([], []);

    let mut functions = FunctionSection::new();
    functions.function(main_type);
    let main_index = abi::IMPORT_COUNT;

    let mut exports = ExportSection::new();
    exports.export("main", ExportKind::Func, main_index);

    let mut main = Function::new([]);
    for instruction in [
        Instruction::I32Const(0), // fn_id of main
        Instruction::Call(Import::Call.func_index()),
        Instruction::I32Const(0), // span_id of `print(42);`
        Instruction::Call(Import::Line.func_index()),
        Instruction::I32Const(42),
        Instruction::Call(Import::Print.func_index()),
        Instruction::I32Const(0), // scope_id of main's body
        Instruction::Call(Import::ScopeExit.func_index()),
        Instruction::I32Const(0), // main returns no value
        Instruction::Call(Import::Ret.func_index()),
        Instruction::End,
    ] {
        main.instruction(&instruction);
    }
    let mut code = CodeSection::new();
    code.function(&main);

    let mut module = Module::new();
    module
        .section(&types)
        .section(&imports)
        .section(&functions)
        .section(&exports)
        .section(&code);

    let debug = DebugTable {
        functions: vec![FunctionInfo {
            name: "main".into(),
            span: Span::new(0, 0),
        }],
        vars: vec![],
        scopes: vec![ScopeInfo {
            fn_id: 0,
            parent: None,
            span: Span::new(0, 0),
        }],
        steps: vec![Span::new(0, 0)],
    };

    CompileOutput {
        wasm: Some(module.finish()),
        debug,
        diagnostics: vec![],
    }
}
