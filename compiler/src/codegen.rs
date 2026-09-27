//! Code generation: checked AST → WebAssembly, with trace instrumentation.
//!
//! **This file is yours to write** (put instrumentation in `instrument.rs` if
//! you like). Spec: `docs/specs/codegen.md`. Tests: `compiler/tests/codegen.rs`
//! and the golden programs in `compiler/tests/programs/`
//! (run with `cargo test --test codegen`).
//! Use `abi::emit_imports` for the trace imports, and look at `stub.rs` for a
//! minimal example of building a module with `wasm-encoder`.

use crate::ast::Program;
use crate::checked::Checked;
use crate::span::Span;

pub struct Output {
    pub wasm: Vec<u8>,
    /// Indexed by `span_id`: the source range to highlight for each
    /// `line(span_id)` call the module makes. Becomes `DebugTable::steps`.
    pub steps: Vec<Span>,
}

/// Compiles a program that passed the checker. Never panics.
pub fn codegen(program: &Program, checked: &Checked) -> Output {
    let _ = (program, checked);
    todo!("write codegen: see docs/specs/codegen.md")
}
