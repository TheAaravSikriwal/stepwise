//! The Stepwise compiler: source text in, WebAssembly plus debug metadata out.
//!
//! Pipeline (DEVPLAN.md §5): lexer → parser → checker → codegen (with trace
//! instrumentation). [`PIPELINE`] says how many of those stages are real; the
//! rest is filled in by the Phase 0 stub, so the playground works (and shows
//! your errors) at every step of the way.

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
mod stub;

pub use debug_table::DebugTable;
pub use diagnostic::{Diagnostic, Severity};
pub use span::{Span, normalize_newlines};

/// How far the real pipeline goes, in order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Stage {
    /// Nothing real yet: every program compiles to `print(42)`.
    Stub,
    /// Lexer errors are reported; valid programs still run the stub.
    Lexer,
    /// Lexer and parser errors are reported.
    Parser,
    /// Every compile error is reported.
    Checker,
    /// The real compiler, end to end.
    Codegen,
}

/// **Bump this when a stage passes its tests** (then run `npm run wasm` in
/// `web/` and try some broken programs in the playground).
pub const PIPELINE: Stage = Stage::Stub;

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
    compile_with(source, PIPELINE)
}

/// [`compile`] with an explicit pipeline stage (for tests and tools).
pub fn compile_with(source: &str, stage: Stage) -> CompileOutput {
    let source = normalize_newlines(source);
    let has_errors = |d: &[Diagnostic]| d.iter().any(Diagnostic::is_error);

    if stage == Stage::Stub {
        return stub::compile(&source);
    }
    if stage == Stage::Lexer {
        let (_, diags) = lexer::lex(&source);
        return if has_errors(&diags) {
            CompileOutput::failed(diags)
        } else {
            stub::compile(&source)
        };
    }

    let (program, diags) = parser::parse(&source);
    if has_errors(&diags) {
        return CompileOutput::failed(diags);
    }
    if stage == Stage::Parser {
        return stub::compile(&source);
    }

    let (checked, diags) = checker::check(&program);
    if has_errors(&diags) {
        return CompileOutput::failed(diags);
    }
    if stage == Stage::Checker {
        return stub::compile(&source);
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
