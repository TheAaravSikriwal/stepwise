//! Lexer: source text → tokens.
//!
//! **This file is yours to write.** The spec is `docs/specs/lexer.md` and the
//! tests are `compiler/tests/lexer.rs` (run them with `cargo test --test lexer`).
//!
//! The token types below are a starting point that the tests use. You may
//! change them; if you do, update the tests to match and tell me, since the
//! parser spec will build on them.

use crate::diagnostic::Diagnostic;
use crate::span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TokenKind {
    // Literals and names
    /// An integer literal, e.g. `42`. At most 2147483648, see the spec.
    Int(u32),
    /// A name: `[A-Za-z_][A-Za-z0-9_]*` that isn't a keyword.
    Ident,

    // Keywords
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    Return,
    True,
    False,

    // Punctuation
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    /// `->`
    Arrow,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    BangEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    /// `&&`
    AndAnd,
    /// `||`
    OrOr,
    /// `!`
    Bang,

    /// Always the last token, with an empty span at the end of the source.
    Eof,
}

/// Splits `source` into tokens.
///
/// Always returns a token list ending in [`TokenKind::Eof`], even when there
/// are errors. Never panics, whatever the input.
pub fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let _ = source;
    todo!("write the lexer: see docs/specs/lexer.md")
}
