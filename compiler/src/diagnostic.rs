//! Compiler errors and warnings, plus a terminal renderer.
//!
//! Build diagnostics with the helpers and keep going: the compiler collects
//! every error it can find instead of stopping at the first one.
//!
//! ```ignore
//! Diagnostic::error("cannot assign twice to immutable variable `x`", assign_span)
//!     .with_label(decl_span, "first assigned here")
//!     .with_note("make it mutable with `let mut x`")
//! ```

use crate::span::{LineIndex, Span};
use serde::Serialize;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// A secondary location that helps explain a diagnostic.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// One plain-English sentence, lowercase, no trailing period.
    pub message: String,
    /// Where the problem is. This is what gets underlined.
    pub span: Span,
    pub labels: Vec<Label>,
    /// Hints and suggestions, shown after the code snippet.
    pub notes: Vec<String>,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: message.into(),
            span,
            labels: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            ..Diagnostic::error(message, span)
        }
    }

    pub fn with_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label {
            span,
            message: message.into(),
        });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    /// Renders like this:
    ///
    /// ```text
    /// error: expected `;` after expression
    ///  --> main.step:3:14
    ///   |
    /// 3 |     let x = 5
    ///   |              ^ expected `;` here
    ///   |
    ///   = note: every statement ends with a semicolon
    /// ```
    ///
    /// Output is plain text (no colors) so it can be snapshot-tested.
    pub fn render(&self, source: &str, file_name: &str) -> String {
        let index = LineIndex::new(source);
        let (line, col) = index.line_col(self.span.start);

        let mut spans: Vec<(Span, &str)> = vec![(self.span, "")];
        spans.extend(self.labels.iter().map(|l| (l.span, l.message.as_str())));
        let gutter = spans
            .iter()
            .map(|(s, _)| index.line_col(s.start).0)
            .max()
            .unwrap_or(line)
            .to_string()
            .len();
        let pad = " ".repeat(gutter);

        let severity = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
        };
        let mut out = String::new();
        let _ = writeln!(out, "{severity}: {}", self.message);
        let _ = writeln!(out, "{pad}--> {file_name}:{line}:{col}");
        let _ = writeln!(out, "{pad} |");

        // Primary span first, then labels in source order.
        spans[1..].sort_by_key(|(s, _)| s.start);
        for (i, (span, message)) in spans.iter().enumerate() {
            let (l, c) = index.line_col(span.start);
            let text = index.line_text(l);
            // A span that crosses lines is underlined to the end of its first line.
            let (end_line, end_col) = index.line_col(span.end);
            let width = if end_line == l {
                (end_col - c).max(1)
            } else {
                (text.chars().count() as u32 + 1 - c).max(1)
            };
            let marker = if i == 0 { "^" } else { "-" };
            let _ = writeln!(out, "{l:>gutter$} | {text}");
            let underline = marker.repeat(width as usize);
            let indent = " ".repeat(c as usize - 1);
            if message.is_empty() {
                let _ = writeln!(out, "{pad} | {indent}{underline}");
            } else {
                let _ = writeln!(out, "{pad} | {indent}{underline} {message}");
            }
        }

        if !self.notes.is_empty() {
            let _ = writeln!(out, "{pad} |");
            for note in &self.notes {
                let _ = writeln!(out, "{pad} = note: {note}");
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_primary_label_and_note() {
        let src = "fn main() {\n    let x = 1;\n    x = 2;\n}\n";
        let assign = src.find("x = 2").unwrap() as u32;
        let decl = src.find("x = 1").unwrap() as u32;
        let d = Diagnostic::error(
            "cannot assign twice to immutable variable `x`",
            Span::new(assign, assign + 5),
        )
        .with_label(Span::new(decl, decl + 1), "declared here without `mut`")
        .with_note("make it mutable: `let mut x = 1;`");

        insta::assert_snapshot!(d.render(src, "main.step"), @r"
        error: cannot assign twice to immutable variable `x`
         --> main.step:3:5
          |
        3 |     x = 2;
          |     ^^^^^
        2 |     let x = 1;
          |         - declared here without `mut`
          |
          = note: make it mutable: `let mut x = 1;`
        ");
    }

    #[test]
    fn empty_span_still_gets_a_caret() {
        let src = "let x = 5\n";
        let d = Diagnostic::error("expected `;` after expression", Span::new(9, 9));
        insta::assert_snapshot!(d.render(src, "main.step"), @r"
        error: expected `;` after expression
         --> main.step:1:10
          |
        1 | let x = 5
          |          ^
        ");
    }
}
