//! Parser tests. Spec: docs/specs/parser.md
//!
//! Run with `cargo test --test parser`. Trees are compared in the compact
//! S-expression form from `ast.rs` (see the table at the top of the printing
//! section there). Suggested order: `shape_*`, then `prec_*`, then `span_*`,
//! then `error_*`, then `robust_*`.

use stepwise_compiler::ast::{ExprKind, Program, StmtKind};
use stepwise_compiler::diagnostic::Diagnostic;
use stepwise_compiler::parser::parse;
use stepwise_compiler::span::Span;

/// Parses input that must have no errors.
fn ok(src: &str) -> Program {
    let (program, diags) = parse(src);
    if !diags.is_empty() {
        let rendered: Vec<String> = diags.iter().map(|d| d.render(src, "test.step")).collect();
        panic!("expected no errors, got:\n{}", rendered.join("\n"));
    }
    program
}

/// The S-expression for a whole program.
fn tree(src: &str) -> String {
    ok(src).to_sexpr()
}

/// The S-expression for the body of `fn main() { <body> }`, without the braces.
fn body(stmts: &str) -> String {
    let t = tree(&format!("fn main() {{ {stmts} }}"));
    let inner = t
        .strip_prefix("(fn main () {")
        .and_then(|s| s.strip_suffix("})"))
        .unwrap_or_else(|| panic!("unexpected tree: {t}"));
    inner.to_string()
}

/// The S-expression for a single expression.
fn expr(e: &str) -> String {
    body(&format!("{e};"))
}

/// Parses input that must produce exactly one error.
fn one_error(src: &str) -> Diagnostic {
    let (_, diags) = parse(src);
    assert_eq!(
        diags.len(),
        1,
        "expected exactly one error for:\n{src}\ngot: {:#?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    diags.into_iter().next().unwrap()
}

fn mentions(d: &Diagnostic, needle: &str) -> bool {
    d.message.contains(needle)
        || d.notes.iter().any(|n| n.contains(needle))
        || d.labels.iter().any(|l| l.message.contains(needle))
}

/// The position just after the first occurrence of `needle`.
fn after(src: &str, needle: &str) -> u32 {
    (src.find(needle)
        .unwrap_or_else(|| panic!("{needle:?} not in source"))
        + needle.len()) as u32
}

// ================================================================ shape

#[test]
fn shape_empty_program() {
    assert_eq!(ok("").functions.len(), 0);
    assert_eq!(ok("  // just a comment\n").functions.len(), 0);
}

#[test]
fn shape_empty_main() {
    assert_eq!(tree("fn main() {}"), "(fn main () {})");
}

#[test]
fn shape_params_and_return_type() {
    assert_eq!(
        tree("fn add(a: int, b: int) -> int { return a + b; }"),
        "(fn add ((a int) (b int)) -> int {(return (+ a b))})"
    );
    assert_eq!(tree("fn f(flag: bool) {}"), "(fn f ((flag bool)) {})");
}

#[test]
fn shape_several_functions() {
    assert_eq!(
        tree("fn a() {}\nfn b() {}\nfn main() {}"),
        "(fn a () {})\n(fn b () {})\n(fn main () {})"
    );
}

#[test]
fn shape_let() {
    assert_eq!(body("let x = 1;"), "(let x 1)");
    assert_eq!(body("let mut x = 1;"), "(let mut x 1)");
    assert_eq!(body("let x: int = 1;"), "(let x: int 1)");
    assert_eq!(
        body("let mut done: bool = false;"),
        "(let mut done: bool false)"
    );
}

#[test]
fn shape_assign() {
    assert_eq!(body("x = x + 1;"), "(= x (+ x 1))");
}

#[test]
fn shape_if_else() {
    assert_eq!(body("if a { f(); }"), "(if a {(call f)})");
    assert_eq!(
        body("if a { f(); } else { g(); }"),
        "(if a {(call f)} {(call g)})"
    );
}

#[test]
fn shape_else_if_chain() {
    // `else if` is an else block containing a single if statement.
    assert_eq!(
        body("if a { f(); } else if b { g(); } else { h(); }"),
        "(if a {(call f)} {(if b {(call g)} {(call h)})})"
    );
}

#[test]
fn shape_while() {
    assert_eq!(
        body("while i < 10 { i = i + 1; }"),
        "(while (< i 10) {(= i (+ i 1))})"
    );
}

#[test]
fn shape_return() {
    assert_eq!(body("return;"), "(return)");
    assert_eq!(body("return 1 + 2;"), "(return (+ 1 2))");
}

#[test]
fn shape_expression_statements() {
    assert_eq!(body("print(x);"), "(call print x)");
    assert_eq!(body("f(); g(1, 2);"), "(call f) (call g 1 2)");
}

#[test]
fn shape_nested_blocks_in_loops() {
    assert_eq!(
        body("while a { if b { return; } }"),
        "(while a {(if b {(return)})})"
    );
}

#[test]
fn shape_the_example_program() {
    let src = include_str!("../../docs/examples/factorial.step");
    assert_eq!(
        tree(src),
        "(fn factorial ((n int)) -> int {(if (<= n 1) {(return 1)}) (return (* n (call factorial (- n 1))))})\n\
         (fn main () {(let mut total 0) (let mut i 1) (while (<= i 5) {(= total (+ total (call factorial i))) (= i (+ i 1))}) (call print total)})"
    );
}

// ================================================================ precedence

#[test]
fn prec_literals_and_names() {
    assert_eq!(expr("42"), "42");
    assert_eq!(expr("true"), "true");
    assert_eq!(expr("false"), "false");
    assert_eq!(expr("x"), "x");
}

#[test]
fn prec_multiplication_binds_tighter_than_addition() {
    assert_eq!(expr("1 + 2 * 3"), "(+ 1 (* 2 3))");
    assert_eq!(expr("1 * 2 + 3"), "(+ (* 1 2) 3)");
    assert_eq!(expr("a % b - c / d"), "(- (% a b) (/ c d))");
}

#[test]
fn prec_parentheses() {
    assert_eq!(expr("(1 + 2) * 3"), "(* (+ 1 2) 3)");
    assert_eq!(expr("((x))"), "x");
}

#[test]
fn prec_left_associative() {
    assert_eq!(expr("1 - 2 - 3"), "(- (- 1 2) 3)");
    assert_eq!(expr("8 / 4 / 2"), "(/ (/ 8 4) 2)");
    assert_eq!(expr("a && b && c"), "(&& (&& a b) c)");
}

#[test]
fn prec_comparison_below_arithmetic() {
    assert_eq!(expr("a + b < c * d"), "(< (+ a b) (* c d))");
    assert_eq!(expr("n % 2 == 0"), "(== (% n 2) 0)");
}

#[test]
fn prec_logic_below_comparison() {
    assert_eq!(expr("a < b && c != d"), "(&& (< a b) (!= c d))");
    assert_eq!(expr("a || b && c"), "(|| a (&& b c))");
    assert_eq!(expr("a && b || c"), "(|| (&& a b) c)");
}

#[test]
fn prec_unary() {
    assert_eq!(expr("!done"), "(! done)");
    assert_eq!(expr("!a && b"), "(&& (! a) b)");
    assert_eq!(expr("-x * 2"), "(* (- x) 2)");
    assert_eq!(expr("!!a"), "(! (! a))");
    assert_eq!(expr("- -x"), "(- (- x))");
}

#[test]
fn prec_negative_literals_are_folded() {
    assert_eq!(expr("-5"), "-5");
    assert_eq!(expr("3 - -5"), "(- 3 -5)");
    assert_eq!(expr("-2147483648"), "-2147483648");
    // Only a literal directly after `-` is folded.
    assert_eq!(expr("-(5)"), "(- 5)");
}

#[test]
fn prec_folded_literal_has_value_and_span() {
    let src = "fn main() { -5; }";
    let program = ok(src);
    let StmtKind::Expr(e) = &program.functions[0].body.stmts[0].kind else {
        panic!("expected an expression statement");
    };
    assert_eq!(e.kind, ExprKind::Int(-5));
    assert_eq!(e.span.text(src), "-5");
}

#[test]
fn prec_calls() {
    assert_eq!(expr("f()"), "(call f)");
    assert_eq!(expr("f(1, 2 + 3)"), "(call f 1 (+ 2 3))");
    assert_eq!(expr("f(g(x), h())"), "(call f (call g x) (call h))");
    assert_eq!(expr("1 + f(2) * 3"), "(+ 1 (* (call f 2) 3))");
}

// ================================================================ spans

fn first_stmt_span(src: &str) -> Span {
    ok(src).functions[0].body.stmts[0].span
}

#[test]
fn span_statement_includes_semicolon() {
    let src = "fn main() { let x = 1 + 2; }";
    assert_eq!(first_stmt_span(src).text(src), "let x = 1 + 2;");
}

#[test]
fn span_binary_expression_covers_both_sides() {
    let src = "fn main() { a * (b + c); }";
    let program = ok(src);
    let StmtKind::Expr(e) = &program.functions[0].body.stmts[0].kind else {
        panic!("expected an expression statement");
    };
    assert_eq!(e.span.text(src), "a * (b + c)");
}

#[test]
fn span_call_covers_through_closing_paren() {
    let src = "fn main() { print(x); }";
    let program = ok(src);
    let StmtKind::Expr(e) = &program.functions[0].body.stmts[0].kind else {
        panic!("expected an expression statement");
    };
    assert_eq!(e.span.text(src), "print(x)");
}

#[test]
fn span_if_covers_else() {
    let src = "fn main() { if a { f(); } else { g(); } }";
    assert_eq!(
        first_stmt_span(src).text(src),
        "if a { f(); } else { g(); }"
    );
}

#[test]
fn span_while_covers_body() {
    let src = "fn main() { while a { f(); } }";
    assert_eq!(first_stmt_span(src).text(src), "while a { f(); }");
}

#[test]
fn span_function_and_names() {
    let src = "fn add(a: int, b: int) -> int { return a; }";
    let f = &ok(src).functions[0];
    assert_eq!(f.span.text(src), src);
    assert_eq!(f.name.span.text(src), "add");
    assert_eq!(f.params[1].name.span.text(src), "b");
    assert_eq!(f.params[1].ty.name.span.text(src), "int");
    assert_eq!(f.body.span.text(src), "{ return a; }");
}

#[test]
fn span_let_name() {
    let src = "fn main() { let mut total = 0; }";
    let program = ok(src);
    let StmtKind::Let { name, .. } = &program.functions[0].body.stmts[0].kind else {
        panic!("expected a let statement");
    };
    assert_eq!(name.span.text(src), "total");
}

// ================================================================ errors

#[test]
fn error_missing_semicolon_points_just_after_previous_token() {
    let src = "fn main() {\n    let x = 5\n    print(x);\n}";
    let d = one_error(src);
    assert!(mentions(&d, "`;`"), "should mention `;`: {}", d.message);
    // Point at the end of `5`, where the `;` belongs, not at `print` on the next line.
    let at = after(src, "= 5");
    assert_eq!(d.span, Span::new(at, at));
}

#[test]
fn error_missing_closing_paren() {
    let src = "fn main() { print(1 + 2; }";
    let d = one_error(src);
    assert!(mentions(&d, "`)`"), "should mention `)`: {}", d.message);
}

#[test]
fn error_unclosed_brace_points_at_the_opening_brace() {
    let src = "fn main() {\n    while true {\n        f();\n}";
    let d = one_error(src);
    assert!(mentions(&d, "`}`"), "should mention `}}`: {}", d.message);
    // The final `}` closes the `while`, so structurally it's main's `{` that is
    // unclosed, though from the indentation the author forgot the while's.
    // Either is accepted (see the spec's stretch goal).
    let brace_at = |needle: &str| {
        let open = after(src, needle) - 1;
        Span::new(open, open + 1)
    };
    let candidates = [brace_at("main() {"), brace_at("while true {")];
    assert!(
        candidates
            .iter()
            .any(|c| d.span == *c || d.labels.iter().any(|l| l.span == *c)),
        "the error or one of its labels should point at an unclosed `{{`"
    );
}

#[test]
fn error_missing_expression() {
    let src = "fn main() { let x = ; }";
    let d = one_error(src);
    let semi = after(src, "= ");
    assert_eq!(
        d.span,
        Span::new(semi, semi + 1),
        "should point at the `;` where a value was expected"
    );
}

#[test]
fn error_let_without_a_name() {
    let src = "fn main() { let 5 = x; }";
    let d = one_error(src);
    assert_eq!(d.span.text(src), "5");
}

#[test]
fn error_chained_comparison() {
    let src = "fn main() { let ok = 1 < x < 5; }";
    let d = one_error(src);
    // Suggest the fix a beginner needs.
    assert!(mentions(&d, "&&"), "should suggest using `&&`: {:?}", d);
}

#[test]
fn error_literal_too_large_without_minus() {
    let src = "fn main() { let x = 2147483648; }";
    let d = one_error(src);
    assert_eq!(d.span.text(src), "2147483648");
}

#[test]
fn error_code_outside_a_function() {
    let src = "let x = 5;\nfn main() {}";
    let d = one_error(src);
    assert!(
        mentions(&d, "fn"),
        "should say code must be inside a function: {:?}",
        d
    );
    // The rest of the file is still parsed.
    let (program, _) = parse(src);
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn error_lexer_errors_are_included() {
    let (_, diags) = parse("fn main() { let x = 5 # 3; }");
    assert!(
        diags.iter().any(|d| d.message.contains('#')),
        "lexer errors should come through parse()"
    );
}

#[test]
fn error_recovery_reports_each_broken_statement_once() {
    // Two independent mistakes: one error each, and no cascade of follow-on errors.
    let src = "fn main() {\n    let x = ;\n    let y = 2\n    print(y);\n}";
    let (_, diags) = parse(src);
    assert_eq!(
        diags.len(),
        2,
        "expected 2 errors, got: {:#?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[test]
fn error_recovery_continues_to_later_functions() {
    let src = "fn broken() { let = ; }\nfn main() { print(1); }";
    let (program, diags) = parse(src);
    assert!(!diags.is_empty());
    assert!(
        program.functions.iter().any(|f| f.name.name == "main"),
        "main should still be parsed after an error in an earlier function"
    );
}

// ================================================================ robustness

fn check_no_panic(src: &str) {
    let result = std::panic::catch_unwind(|| {
        let (program, diags) = parse(src);
        for d in &diags {
            assert!(
                d.span.end as usize <= src.len(),
                "diagnostic span out of bounds"
            );
        }
        let _ = program.to_sexpr();
    });
    if result.is_err() {
        panic!("parser panicked on input:\n{src}");
    }
}

// Very deep nesting is tested in parser_depth.rs, because a stack overflow
// aborts the whole test binary and would hide every result in this file.

#[test]
fn robust_moderate_nesting_is_fine() {
    let depth = 50;
    let src = format!(
        "fn main() {{ let x = {}1{}; }}",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    ok(&src);
}

#[test]
fn robust_never_panics_on_random_token_soup() {
    let pieces = [
        "fn", "main", "(", ")", "{", "}", "let", "mut", "x", "=", "1", ";", "if", "else", "while",
        "return", "+", "-", "*", "<", "==", "&&", "!", ",", ":", "->", "int", "true", "print", "#",
        "\n",
    ];
    let mut seed: u64 = 0xfeed;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for _ in 0..3000 {
        let len = (next() % 30) as usize;
        let src: Vec<&str> = (0..len)
            .map(|_| pieces[(next() % pieces.len() as u64) as usize])
            .collect();
        check_no_panic(&src.join(" "));
    }
}

#[test]
fn robust_never_panics_when_deleting_any_token_from_the_example() {
    // Every "one token missing" variant of a real program: the most common
    // kind of beginner typo.
    let src = include_str!("../../docs/examples/factorial.step");
    let (tokens, _) = stepwise_compiler::lexer::lex(src);
    for t in &tokens {
        let (s, e) = (t.span.start as usize, t.span.end as usize);
        let broken = format!("{}{}", &src[..s], &src[e..]);
        check_no_panic(&broken);
    }
}
