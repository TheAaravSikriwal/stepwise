//! Lexer tests. Spec: docs/specs/lexer.md
//!
//! Run with `cargo test --test lexer`. Start with the `basics_*` tests and
//! work down; the `robust_*` tests at the end are the hardest.

use stepwise_compiler::diagnostic::Diagnostic;
use stepwise_compiler::lexer::{Token, TokenKind, lex};
use stepwise_compiler::span::Span;

use TokenKind::*;

/// Token kinds, without the trailing `Eof`, for inputs that must lex cleanly.
fn kinds(src: &str) -> Vec<TokenKind> {
    let (tokens, diags) = lex(src);
    assert_no_errors(src, &diags);
    assert_eq!(
        tokens.last().map(|t| t.kind),
        Some(Eof),
        "last token must be Eof"
    );
    tokens[..tokens.len() - 1].iter().map(|t| t.kind).collect()
}

/// `(kind, source text)` pairs, without the trailing `Eof`.
fn texts(src: &str) -> Vec<(TokenKind, &str)> {
    let (tokens, diags) = lex(src);
    assert_no_errors(src, &diags);
    tokens[..tokens.len() - 1]
        .iter()
        .map(|t| (t.kind, t.span.text(src)))
        .collect()
}

fn assert_no_errors(src: &str, diags: &[Diagnostic]) {
    if !diags.is_empty() {
        let rendered: Vec<String> = diags.iter().map(|d| d.render(src, "test.step")).collect();
        panic!("expected no errors, got:\n{}", rendered.join("\n"));
    }
}

/// Lexes input that must produce exactly one error; returns it and the tokens.
fn one_error(src: &str) -> (Diagnostic, Vec<TokenKind>) {
    let (tokens, diags) = lex(src);
    assert_eq!(
        diags.len(),
        1,
        "expected exactly one error for {src:?}, got {diags:#?}"
    );
    let kinds = tokens.iter().map(|t| t.kind).collect();
    (diags.into_iter().next().unwrap(), kinds)
}

// ---------------------------------------------------------------- basics

#[test]
fn basics_empty_input_is_just_eof() {
    let (tokens, diags) = lex("");
    assert!(diags.is_empty());
    assert_eq!(
        tokens,
        [Token {
            kind: Eof,
            span: Span::new(0, 0)
        }]
    );
}

#[test]
fn basics_eof_span_is_at_end_of_source() {
    let src = "x  ";
    let (tokens, _) = lex(src);
    assert_eq!(tokens.last().unwrap().span, Span::new(3, 3));
}

#[test]
fn basics_single_char_punctuation() {
    assert_eq!(
        kinds("( ) { } , ; : + - * / % = < > !"),
        [
            LParen, RParen, LBrace, RBrace, Comma, Semicolon, Colon, Plus, Minus, Star, Slash,
            Percent, Eq, Lt, Gt, Bang
        ]
    );
}

#[test]
fn basics_two_char_operators() {
    assert_eq!(
        kinds("-> == != <= >= && ||"),
        [Arrow, EqEq, BangEq, LtEq, GtEq, AndAnd, OrOr]
    );
}

#[test]
fn basics_longest_match_wins() {
    // `==` is one token, not two `=`; `<=` is one token; `===` is `==` then `=`.
    assert_eq!(kinds("a==b"), [Ident, EqEq, Ident]);
    assert_eq!(kinds("a<=b"), [Ident, LtEq, Ident]);
    assert_eq!(kinds("==="), [EqEq, Eq]);
    assert_eq!(kinds("!!="), [Bang, BangEq]);
    assert_eq!(kinds("a-->b"), [Ident, Minus, Arrow, Ident]);
}

#[test]
fn basics_spans_cover_exact_text() {
    assert_eq!(
        texts("let  x=10;"),
        [
            (Let, "let"),
            (Ident, "x"),
            (Eq, "="),
            (Int(10), "10"),
            (Semicolon, ";")
        ]
    );
}

// ---------------------------------------------------------------- keywords and identifiers

#[test]
fn keywords() {
    assert_eq!(
        kinds("fn let mut if else while return true false"),
        [Fn, Let, Mut, If, Else, While, Return, True, False]
    );
}

#[test]
fn identifiers() {
    assert_eq!(
        texts("x total_2 _tmp CamelCase"),
        [
            (Ident, "x"),
            (Ident, "total_2"),
            (Ident, "_tmp"),
            (Ident, "CamelCase")
        ]
    );
}

#[test]
fn keyword_prefixes_are_identifiers() {
    // A keyword only counts if the whole word matches.
    assert_eq!(
        kinds("fnord letter iffy mutable returned truely"),
        [Ident, Ident, Ident, Ident, Ident, Ident]
    );
}

#[test]
fn keywords_are_case_sensitive() {
    assert_eq!(kinds("Fn LET True"), [Ident, Ident, Ident]);
}

#[test]
fn type_names_and_print_are_plain_identifiers() {
    // `int`, `bool` and `print` are names, not keywords (see the spec for why).
    assert_eq!(kinds("int bool print"), [Ident, Ident, Ident]);
}

// ---------------------------------------------------------------- numbers

#[test]
fn integers() {
    assert_eq!(
        kinds("0 7 42 1000000"),
        [Int(0), Int(7), Int(42), Int(1_000_000)]
    );
}

#[test]
fn leading_zeros_are_allowed() {
    assert_eq!(kinds("007"), [Int(7)]);
}

#[test]
fn negative_numbers_are_minus_then_int() {
    // The parser turns these into unary minus, not the lexer.
    assert_eq!(kinds("-5"), [Minus, Int(5)]);
    assert_eq!(kinds("x-5"), [Ident, Minus, Int(5)]);
}

#[test]
fn largest_allowed_literals() {
    assert_eq!(kinds("2147483647"), [Int(2_147_483_647)]);
    // Allowed so that `-2147483648` (the smallest int) can be written.
    // The parser rejects it without a minus sign in front.
    assert_eq!(kinds("2147483648"), [Int(2_147_483_648)]);
}

#[test]
fn too_large_literal_is_an_error() {
    let src = "let x = 99999999999999999999;";
    let (d, kinds) = one_error(src);
    assert_eq!(d.span.text(src), "99999999999999999999");
    assert!(
        d.message.contains("too large") || d.message.contains("too big"),
        "message should say the number is too large, got: {}",
        d.message
    );
    // Lexing continues after the error.
    assert_eq!(kinds.last(), Some(&Eof));
    assert!(kinds.contains(&Semicolon));
}

#[test]
fn just_over_the_limit_is_an_error() {
    let (d, _) = one_error("2147483649");
    assert_eq!(d.span, Span::new(0, 10));
}

// ---------------------------------------------------------------- whitespace and comments

#[test]
fn whitespace_is_skipped() {
    assert_eq!(kinds(" \t\n x \n\n\t y \n"), [Ident, Ident]);
}

#[test]
fn line_comments_are_skipped() {
    let src = "x // this is ignored: fn let ### é\ny";
    assert_eq!(kinds(src), [Ident, Ident]);
}

#[test]
fn comment_at_end_of_file_without_newline() {
    assert_eq!(kinds("x // no newline after me"), [Ident]);
}

#[test]
fn slash_alone_is_division() {
    assert_eq!(kinds("a / b"), [Ident, Slash, Ident]);
}

// ---------------------------------------------------------------- errors

#[test]
fn unexpected_character() {
    let src = "let x = 5 # 3;";
    let (d, kinds) = one_error(src);
    assert_eq!(d.span.text(src), "#");
    assert!(
        d.message.contains('#'),
        "message should show the character: {}",
        d.message
    );
    // The bad character is skipped and lexing continues.
    assert_eq!(kinds, [Let, Ident, Eq, Int(5), Int(3), Semicolon, Eof]);
}

#[test]
fn every_bad_character_is_reported() {
    let (tokens, diags) = lex("a @ b $ c");
    assert_eq!(diags.len(), 2);
    let kinds: Vec<_> = tokens.iter().map(|t| t.kind).collect();
    assert_eq!(kinds, [Ident, Ident, Ident, Eof]);
}

#[test]
fn single_ampersand_suggests_double() {
    let src = "a & b";
    let (d, _) = one_error(src);
    assert_eq!(d.span.text(src), "&");
    let all_text = format!("{} {}", d.message, d.notes.join(" "));
    assert!(
        all_text.contains("&&"),
        "should suggest `&&`, got: {all_text}"
    );
}

#[test]
fn single_pipe_suggests_double() {
    let src = "a | b";
    let (d, _) = one_error(src);
    assert_eq!(d.span.text(src), "|");
    let all_text = format!("{} {}", d.message, d.notes.join(" "));
    assert!(
        all_text.contains("||"),
        "should suggest `||`, got: {all_text}"
    );
}

// ---------------------------------------------------------------- robustness

#[test]
fn robust_non_ascii_character_is_one_error_covering_the_whole_char() {
    // 'é' is 2 bytes in UTF-8. The span must cover both, or slicing panics.
    let src = "let é = 1;";
    let (d, _) = one_error(src);
    assert_eq!(d.span.text(src), "é");
}

#[test]
fn robust_emoji() {
    let src = "x 😀 y";
    let (d, kinds) = one_error(src);
    assert_eq!(d.span.text(src), "😀");
    assert_eq!(kinds, [Ident, Ident, Eof]);
}

#[test]
fn robust_the_example_program() {
    let src = include_str!("../../docs/examples/factorial.step");
    let (tokens, diags) = lex(src);
    assert_no_errors(src, &diags);
    assert_eq!(
        tokens.len(),
        75,
        "token count for factorial.step (including Eof)"
    );
    // Spot checks
    assert_eq!(tokens[0].kind, Fn);
    assert_eq!(tokens[1].span.text(src), "factorial");
    assert_eq!(tokens[tokens.len() - 1].kind, Eof);
}

/// Checks the invariants every token list must satisfy, for any input.
fn check_invariants(src: &str) {
    let (tokens, _) = lex(src);
    let last = tokens.last().expect("token list must not be empty");
    assert_eq!(last.kind, Eof);
    assert_eq!(last.span, Span::new(src.len() as u32, src.len() as u32));
    let mut prev_end = 0;
    for t in &tokens {
        let (s, e) = (t.span.start as usize, t.span.end as usize);
        assert!(
            s <= e && e <= src.len(),
            "span {s}..{e} out of bounds in {src:?}"
        );
        assert!(s >= prev_end, "tokens overlap or go backwards in {src:?}");
        assert!(
            src.is_char_boundary(s) && src.is_char_boundary(e),
            "span {s}..{e} splits a character in {src:?}"
        );
        if t.kind != Eof {
            assert!(
                s < e,
                "only Eof may have an empty span ({:?} in {src:?})",
                t.kind
            );
        }
        prev_end = e;
    }
}

#[test]
fn robust_never_panics_on_random_input() {
    // A tiny deterministic random generator, so failures are reproducible.
    let mut seed: u64 = 0x5eed;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let alphabet: Vec<char> = "abz_09 \t\n(){},;:+-*/%=<>!&|#@\"'.é😀\r".chars().collect();
    for _ in 0..2000 {
        let len = (next() % 24) as usize;
        let src: String = (0..len)
            .map(|_| alphabet[(next() % alphabet.len() as u64) as usize])
            .collect();
        let result = std::panic::catch_unwind(|| check_invariants(&src));
        if result.is_err() {
            panic!("lexer failed on input {src:?} (see the panic message above)");
        }
    }
}
