//! The Stepwise compiler: source text in, WebAssembly plus debug metadata out.
//!
//! Pipeline (DEVPLAN.md §5): lexer → parser → resolver/type checker → codegen
//! (with trace instrumentation). Until those stages exist, [`compile`] returns
//! a hard-coded module (see `stub.rs`) so the rest of the system can be built
//! and tested end to end.

pub mod abi;
pub mod debug_table;
pub mod diagnostic;
pub mod lexer;
pub mod span;
mod stub;

pub use debug_table::DebugTable;
pub use diagnostic::{Diagnostic, Severity};
pub use span::{Span, normalize_newlines};

#[derive(Debug)]
pub struct CompileOutput {
    /// The compiled module, or `None` if any diagnostic is an error.
    pub wasm: Option<Vec<u8>>,
    pub debug: DebugTable,
    /// All errors and warnings found, in source order.
    pub diagnostics: Vec<Diagnostic>,
}

impl CompileOutput {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Compiles a Stepwise program. Never panics on bad input: every problem
/// becomes a [`Diagnostic`].
pub fn compile(source: &str) -> CompileOutput {
    let source = normalize_newlines(source);
    stub::compile(&source)
}
