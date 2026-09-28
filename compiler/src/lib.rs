//! The Stepwise compiler: source text in, WebAssembly plus debug metadata out.
//!
//! Pipeline (DEVPLAN.md §5): lexer → parser → checker → codegen (with trace
//! instrumentation). Each stage only runs if the one before found no errors:
//! checking a program that didn't parse, or compiling one that didn't check,
//! would only produce confusing follow-on errors.

pub mod abi;
pub mod ast;
pub mod checked;
pub mod checker;
pub mod codegen;
pub mod debug_table;
pub mod diagnostic;
pub mod lexer;
pub mod parser;
pub mod span;

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

    fn failed(mut diagnostics: Vec<Diagnostic>) -> Self {
        diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
        CompileOutput {
            wasm: None,
            debug: DebugTable::default(),
            diagnostics,
        }
    }
}

/// Compiles a Stepwise program. Never panics on bad input: every problem
/// becomes a [`Diagnostic`].
pub fn compile(source: &str) -> CompileOutput {
    let source = normalize_newlines(source);
    let has_errors = |d: &[Diagnostic]| d.iter().any(Diagnostic::is_error);

    let (program, diags) = parser::parse(&source);
    if has_errors(&diags) {
        return CompileOutput::failed(diags);
    }

    let (checked, diags) = checker::check(&program);
    if has_errors(&diags) {
        return CompileOutput::failed(diags);
    }

    let out = codegen::codegen(&program, &checked);
    let mut debug = checked.debug;
    debug.steps = out.steps;
    CompileOutput {
        wasm: Some(out.wasm),
        debug,
        diagnostics: diags,
    }
}
