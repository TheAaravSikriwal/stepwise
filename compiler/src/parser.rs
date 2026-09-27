//! Parser: tokens → abstract syntax tree.
//!
//! **This file is yours to write.** The spec is `docs/specs/parser.md` and the
//! tests are `compiler/tests/parser.rs` (run them with `cargo test --test parser`).
//! The tree types are in `ast.rs`.

use crate::ast::Program;
use crate::diagnostic::Diagnostic;

/// Lexes and parses `source`.
///
/// Returns every diagnostic from both the lexer and the parser. Always
/// returns a `Program`, even with errors (containing whatever parsed
/// successfully). Never panics, whatever the input.
pub fn parse(source: &str) -> (Program, Vec<Diagnostic>) {
    let _ = source;
    todo!("write the parser: see docs/specs/parser.md")
}
