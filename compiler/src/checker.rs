//! Checker: name resolution and type checking.
//!
//! **This file is yours to write** (split it into `resolve.rs` and
//! `types.rs` if you prefer). Spec: `docs/specs/checker.md`.
//! Tests: `compiler/tests/checker.rs` (run with `cargo test --test checker`).
//! Output types: `checked.rs`.

use crate::ast::Program;
use crate::checked::Checked;
use crate::diagnostic::Diagnostic;

/// Checks a program that parsed without errors.
///
/// Reports every problem it can find, never just the first. Never panics.
pub fn check(program: &Program) -> (Checked, Vec<Diagnostic>) {
    let _ = program;
    todo!("write the checker: see docs/specs/checker.md")
}
