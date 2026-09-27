//! Lexer: source text → tokens.
//!
//! Spec: `docs/specs/lexer.md`. Tests: `compiler/tests/lexer.rs`.
//!
//! The lexer walks the source once, byte by byte. Every token Stepwise has is
//! ASCII, so bytes are all it needs; the only time it decodes a full `char`
//! is to report an unexpected non-ASCII character, so the error's span covers
//! the whole character (slicing mid-character would panic).
//!
//! It never stops at an error: the bad text is reported and skipped, and
//! lexing carries on, so one typo produces one error.

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
    let mut lexer = Lexer {
        source,
        bytes: source.as_bytes(),
        pos: 0,
        tokens: Vec::new(),
        diagnostics: Vec::new(),
    };
    lexer.run();
    let end = source.len() as u32;
    lexer.tokens.push(Token {
        kind: TokenKind::Eof,
        span: Span::new(end, end),
    });
    (lexer.tokens, lexer.diagnostics)
}

/// The largest integer literal allowed. One more than `i32::MAX`, so that
/// `-2147483648` (the smallest int) can be written: the parser folds the
/// minus in, and rejects 2147483648 anywhere else.
const MAX_LITERAL: u64 = 2_147_483_648;

struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    /// Byte offset of the next unread byte.
    pos: usize,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
}

impl Lexer<'_> {
    fn run(&mut self) {
        while let Some(&byte) = self.bytes.get(self.pos) {
            let start = self.pos;
            match byte {
                b' ' | b'\t' | b'\n' => self.pos += 1,
                b'/' if self.peek_at(1) == Some(b'/') => self.skip_comment(),
                b'0'..=b'9' => self.number(start),
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.word(start),
                _ => self.symbol(start, byte),
            }
        }
    }

    fn peek_at(&self, ahead: usize) -> Option<u8> {
        self.bytes.get(self.pos + ahead).copied()
    }

    fn span(&self, start: usize) -> Span {
        Span::new(start as u32, self.pos as u32)
    }

    fn push(&mut self, kind: TokenKind, start: usize) {
        let span = self.span(start);
        self.tokens.push(Token { kind, span });
    }

    /// `//` to the end of the line. The newline itself is left as whitespace.
    fn skip_comment(&mut self) {
        while self.bytes.get(self.pos).is_some_and(|&b| b != b'\n') {
            self.pos += 1;
        }
    }

    /// A run of digits. All of them are consumed before the range check, so
    /// a too-large number is one error covering the whole number. The value
    /// saturates rather than overflowing while it's read.
    fn number(&mut self, start: usize) {
        let mut value: u64 = 0;
        while let Some(&b) = self.bytes.get(self.pos).filter(|b| b.is_ascii_digit()) {
            value = value.saturating_mul(10).saturating_add(u64::from(b - b'0'));
            self.pos += 1;
        }
        if value > MAX_LITERAL {
            let span = self.span(start);
            self.diagnostics.push(
                Diagnostic::error("this number is too large for an int", span)
                    .with_note("the largest int is 2147483647"),
            );
        } else {
            self.push(TokenKind::Int(value as u32), start);
        }
    }

    /// An identifier or a keyword. A keyword has to be the whole word, so
    /// `letter` is an identifier, not `let` followed by `ter`.
    fn word(&mut self, start: usize) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            self.pos += 1;
        }
        let kind = match &self.source[start..self.pos] {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "return" => TokenKind::Return,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            _ => TokenKind::Ident,
        };
        self.push(kind, start);
    }

    /// Punctuation and operators. Two-character operators are tried first,
    /// because the longest match wins: `==` is one token, not two `=`.
    fn symbol(&mut self, start: usize, byte: u8) {
        use TokenKind::*;
        let next = self.peek_at(1);
        let (kind, len) = match (byte, next) {
            (b'-', Some(b'>')) => (Arrow, 2),
            (b'=', Some(b'=')) => (EqEq, 2),
            (b'!', Some(b'=')) => (BangEq, 2),
            (b'<', Some(b'=')) => (LtEq, 2),
            (b'>', Some(b'=')) => (GtEq, 2),
            (b'&', Some(b'&')) => (AndAnd, 2),
            (b'|', Some(b'|')) => (OrOr, 2),
            (b'(', _) => (LParen, 1),
            (b')', _) => (RParen, 1),
            (b'{', _) => (LBrace, 1),
            (b'}', _) => (RBrace, 1),
            (b',', _) => (Comma, 1),
            (b';', _) => (Semicolon, 1),
            (b':', _) => (Colon, 1),
            (b'+', _) => (Plus, 1),
            (b'-', _) => (Minus, 1),
            (b'*', _) => (Star, 1),
            (b'/', _) => (Slash, 1),
            (b'%', _) => (Percent, 1),
            (b'=', _) => (Eq, 1),
            (b'<', _) => (Lt, 1),
            (b'>', _) => (Gt, 1),
            (b'!', _) => (Bang, 1),
            (b'&' | b'|', _) => return self.lone_logic_operator(start, byte),
            _ => return self.unexpected(start),
        };
        self.pos += len;
        self.push(kind, start);
    }

    /// A single `&` or `|`: almost certainly a mistyped `&&` or `||`.
    fn lone_logic_operator(&mut self, start: usize, byte: u8) {
        self.pos += 1;
        let (op, word) = if byte == b'&' {
            ("&&", "and")
        } else {
            ("||", "or")
        };
        let span = self.span(start);
        self.diagnostics.push(
            Diagnostic::error(format!("unexpected `{}`", byte as char), span)
                .with_note(format!("for \"{word}\", use `{op}`")),
        );
    }

    /// Anything else. Decodes the whole character (it may be several bytes
    /// long) so the error points at, and skips, exactly that character.
    fn unexpected(&mut self, start: usize) {
        let ch = self.source[start..].chars().next().unwrap_or('\u{fffd}');
        self.pos += ch.len_utf8().max(1);
        let span = self.span(start);
        let shown = match ch {
            '\r' => "a carriage return (\\r)".to_string(),
            c if c.is_control() => format!("the control character U+{:04X}", c as u32),
            c => format!("`{c}`"),
        };
        self.diagnostics.push(
            Diagnostic::error(format!("unexpected character {shown}"), span)
                .with_note("Stepwise code only uses letters, digits, `_`, and the symbols ( ) { } , ; : + - * / % = < > ! & |"),
        );
    }
}
