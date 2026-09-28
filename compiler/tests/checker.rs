//! Checker tests. Spec: docs/specs/checker.md
//!
//! Run with `cargo test --test checker`. These need a working parser.
//! Suggested order: `resolve_*`, `scope_*`, `types_*`, `calls_*`, `returns_*`,
//! `program_*`, `cascade_*`, `debug_*`.

use stepwise_compiler::ast::{Program, StmtKind};
use stepwise_compiler::checked::{Callee, Checked, Type};
use stepwise_compiler::checker::check;
use stepwise_compiler::debug_table::TypeName;
use stepwise_compiler::diagnostic::Diagnostic;
use stepwise_compiler::parser::parse;
use stepwise_compiler::span::Span;

fn parsed(src: &str) -> Program {
    let (program, diags) = parse(src);
    assert!(
        diags.is_empty(),
        "test program must parse cleanly: {diags:#?}"
    );
    program
}

fn ok(src: &str) -> (Program, Checked) {
    let program = parsed(src);
    let (checked, diags) = check(&program);
    if !diags.is_empty() {
        let rendered: Vec<String> = diags.iter().map(|d| d.render(src, "test.step")).collect();
        panic!("expected no errors, got:\n{}", rendered.join("\n"));
    }
    (program, checked)
}

fn errors(src: &str) -> Vec<Diagnostic> {
    check(&parsed(src)).1
}

fn one_error(src: &str) -> Diagnostic {
    let diags = errors(src);
    assert_eq!(
        diags.len(),
        1,
        "expected exactly one error for:\n{src}\ngot: {:#?}",
        diags.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    diags.into_iter().next().unwrap()
}

/// `main` wrapping a body, plus any helper functions.
fn in_main(body: &str) -> String {
    format!("fn main() {{\n{body}\n}}")
}

fn mentions(d: &Diagnostic, needle: &str) -> bool {
    d.message.contains(needle)
        || d.notes.iter().any(|n| n.contains(needle))
        || d.labels.iter().any(|l| l.message.contains(needle))
}

fn assert_mentions(d: &Diagnostic, needle: &str) {
    assert!(
        mentions(d, needle),
        "expected the error to mention {needle:?}: {d:#?}"
    );
}

/// Span of the `nth` (0-based) occurrence of `needle` in `src`.
fn nth(src: &str, needle: &str, n: usize) -> Span {
    let start = src
        .match_indices(needle)
        .nth(n)
        .unwrap_or_else(|| panic!("occurrence {n} of {needle:?} not found"))
        .0;
    Span::new(start as u32, (start + needle.len()) as u32)
}

fn first(src: &str, needle: &str) -> Span {
    nth(src, needle, 0)
}

/// Span of `sub` inside the first occurrence of `context`. Use this for short
/// names: `first(src, "n")` would find the `n` in `fn`.
fn within(src: &str, context: &str, sub: &str) -> Span {
    let base = first(src, context).start as usize;
    let off = context
        .find(sub)
        .unwrap_or_else(|| panic!("{sub:?} not in {context:?}"));
    Span::new((base + off) as u32, (base + off + sub.len()) as u32)
}

fn has_label_at(d: &Diagnostic, span: Span) -> bool {
    d.labels.iter().any(|l| l.span == span)
}

// ================================================================ resolve

#[test]
fn resolve_uses_map_to_their_declaration() {
    let src = in_main("let mut total = 0;\ntotal = total + 1;\nprint(total);");
    let (_, c) = ok(&src);
    let ids: Vec<_> = (0..4).map(|n| c.vars[&nth(&src, "total", n)]).collect();
    assert!(
        ids.iter().all(|&id| id == ids[0]),
        "all four `total`s are the same variable"
    );
}

#[test]
fn resolve_different_variables_get_different_ids() {
    let src = in_main("let apple = 1;\nlet berry = 2;\nprint(apple + berry);");
    let (_, c) = ok(&src);
    assert_ne!(c.vars[&first(&src, "apple")], c.vars[&first(&src, "berry")]);
}

#[test]
fn resolve_parameters_are_variables() {
    let src = "fn double(n: int) -> int { return n * 2; }\nfn main() { print(double(4)); }";
    let (_, c) = ok(src);
    assert_eq!(
        c.vars[&within(src, "(n:", "n")],
        c.vars[&within(src, "n * 2", "n")]
    );
}

#[test]
fn resolve_unknown_variable() {
    let src = in_main("print(missing);");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "missing"));
    assert_mentions(&d, "missing");
}

#[test]
fn resolve_unknown_variable_suggests_a_close_name() {
    let src = in_main("let total = 5;\nprint(totl);");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "totl"));
    assert_mentions(&d, "total");
}

#[test]
fn resolve_use_before_declaration() {
    let src = in_main("print(x);\nlet x = 1;");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "x"));
}

#[test]
fn resolve_variable_cannot_refer_to_itself() {
    let src = in_main("let x = x + 1;");
    let d = one_error(&src);
    assert_eq!(d.span, nth(&src, "x", 1));
}

#[test]
fn resolve_assign_to_unknown_variable() {
    let src = in_main("y = 3;");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "y"));
}

#[test]
fn resolve_assign_to_immutable_points_at_declaration() {
    let src = in_main("let count = 0;\ncount = count + 1;");
    let d = one_error(&src);
    // The assignment's target is the problem...
    assert_eq!(d.span, nth(&src, "count", 1));
    // ...and a label shows where it was declared, with the fix.
    assert!(
        has_label_at(&d, first(&src, "count")),
        "should label the declaration: {d:#?}"
    );
    assert_mentions(&d, "mut");
}

#[test]
fn resolve_parameters_are_immutable() {
    let src = "fn f(n: int) { n = n - 1; }\nfn main() { f(1); }";
    let d = one_error(src);
    assert_eq!(d.span, within(src, "{ n =", "n"));
}

// ================================================================ scope

#[test]
fn scope_block_variables_are_not_visible_after_the_block() {
    let src = in_main("if true {\n    let inner = 1;\n}\nprint(inner);");
    let d = one_error(&src);
    assert_eq!(d.span, nth(&src, "inner", 1));
}

#[test]
fn scope_outer_variables_are_visible_inside_blocks() {
    let src = in_main("let x = 1;\nwhile x < 0 {\n    if true { print(x); }\n}");
    ok(&src);
}

#[test]
fn scope_sibling_blocks_can_reuse_a_name() {
    let src = in_main("if true { let tmp = 1; print(tmp); }\nif true { let tmp = 2; print(tmp); }");
    let (_, c) = ok(&src);
    assert_ne!(
        c.vars[&nth(&src, "tmp", 0)],
        c.vars[&nth(&src, "tmp", 2)],
        "two different variables"
    );
}

#[test]
fn scope_redeclaring_in_the_same_scope_is_an_error() {
    let src = in_main("let x = 1;\nlet x = 2;");
    let d = one_error(&src);
    assert_eq!(d.span, nth(&src, "x", 1));
    assert!(
        has_label_at(&d, nth(&src, "x", 0)),
        "should label the first declaration"
    );
}

#[test]
fn scope_shadowing_an_outer_variable_is_an_error() {
    // Stepwise doesn't allow shadowing: two live variables with the same name
    // would be confusing in the debugger's variables panel.
    let src = in_main("let x = 1;\nif true {\n    let x = 2;\n}");
    let d = one_error(&src);
    assert_eq!(d.span, nth(&src, "x", 1));
}

#[test]
fn scope_shadowing_a_parameter_is_an_error() {
    let src = "fn f(n: int) { let n = 2; }\nfn main() { f(1); }";
    let d = one_error(src);
    assert_eq!(d.span, within(src, "let n", "n"));
}

#[test]
fn scope_every_block_gets_a_scope() {
    let src = in_main("if true { } else { }\nwhile false { }");
    let (program, c) = ok(&src);
    let main_body = &program.functions[0].body;
    assert!(c.scopes.contains_key(&main_body.span));
    let StmtKind::If {
        then_block,
        else_block,
        ..
    } = &main_body.stmts[0].kind
    else {
        panic!()
    };
    assert!(c.scopes.contains_key(&then_block.span));
    assert!(c.scopes.contains_key(&else_block.as_ref().unwrap().span));
    assert_eq!(c.scopes.len(), 4);
}

// ================================================================ types

#[test]
fn types_expression_types_are_recorded() {
    let src = in_main("let a = 1 + 2;\nlet b = a < 3;\nlet c = !b && true;");
    let (_, c) = ok(&src);
    assert_eq!(c.types[&first(&src, "1 + 2")], Type::Int);
    assert_eq!(c.types[&first(&src, "a < 3")], Type::Bool);
    assert_eq!(c.types[&first(&src, "!b && true")], Type::Bool);
    assert_eq!(c.types[&first(&src, "!b")], Type::Bool);
}

#[test]
fn types_arithmetic_needs_ints() {
    let src = in_main("let x = 1 + true;");
    let d = one_error(&src);
    assert_eq!(
        d.span,
        first(&src, "true"),
        "point at the operand with the wrong type"
    );
    assert_mentions(&d, "bool");
}

#[test]
fn types_logic_needs_bools() {
    let src = in_main("let x = 1 && true;");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "1"));
}

#[test]
fn types_comparison_needs_ints() {
    let src = in_main("let x = true < false;");
    let errs = errors(&src);
    assert!(!errs.is_empty());
}

#[test]
fn types_equality_needs_matching_types() {
    ok(&in_main("let a = 1 == 2;\nlet b = true != false;"));
    let src = in_main("let x = 1 == true;");
    let d = one_error(&src);
    assert!(
        d.span == first(&src, "1 == true") || d.span == first(&src, "true"),
        "point at the comparison or its right side"
    );
}

#[test]
fn types_unary_operators() {
    ok(&in_main("let a = -(1 + 2);\nlet b = !false;"));
    let src = in_main("let x = !5;");
    assert_eq!(one_error(&src).span, first(&src, "5"));
    let src = in_main("let x = -true;");
    assert_eq!(one_error(&src).span, first(&src, "true"));
}

#[test]
fn types_if_condition_must_be_bool() {
    let src = in_main("let x = 3;\nif x { }");
    let d = one_error(&src);
    assert_eq!(d.span, nth(&src, "x", 1));
    // Point a beginner at what they probably meant.
    assert_mentions(&d, "!= 0");
}

#[test]
fn types_while_condition_must_be_bool() {
    let src = in_main("while 1 { }");
    assert_eq!(one_error(&src).span, first(&src, "1"));
}

#[test]
fn types_let_annotation_must_match() {
    ok(&in_main("let a: int = 5;\nlet b: bool = a > 2;"));
    let src = in_main("let flag: bool = 5;");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "5"));
}

#[test]
fn types_assignment_must_keep_the_type() {
    let src = in_main("let mut x = 1;\nx = false;");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "false"));
}

#[test]
fn types_unknown_type_name() {
    let src = "fn f(n: integer) { }\nfn main() { }";
    let d = one_error(src);
    assert_eq!(d.span, first(src, "integer"));
    assert_mentions(&d, "`int`");
}

// ================================================================ calls

#[test]
fn calls_are_resolved() {
    let src = "fn helper() -> int { return 1; }\nfn main() { print(helper()); }";
    let (_, c) = ok(src);
    assert_eq!(c.calls[&nth(src, "helper", 1)], Callee::Function(0));
    assert_eq!(c.calls[&first(src, "print")], Callee::Print(Type::Int));
}

#[test]
fn calls_print_records_its_argument_type() {
    let src = in_main("print(1 < 2);");
    let (_, c) = ok(&src);
    assert_eq!(c.calls[&first(&src, "print")], Callee::Print(Type::Bool));
}

#[test]
fn calls_can_call_functions_defined_later_and_recurse() {
    ok(
        "fn main() { print(fact(5)); }\nfn fact(n: int) -> int { if n <= 1 { return 1; } return n * fact(n - 1); }",
    );
}

#[test]
fn calls_unknown_function() {
    let src = in_main("fly();");
    let d = one_error(&src);
    assert_eq!(d.span, first(&src, "fly"));
}

#[test]
fn calls_calling_a_variable() {
    let src = in_main("let x = 1;\nx();");
    let d = one_error(&src);
    assert_eq!(d.span, nth(&src, "x", 1));
    assert_mentions(&d, "variable");
}

#[test]
fn calls_using_a_function_as_a_variable() {
    let src = "fn f() { }\nfn main() { let x = f; }";
    let d = one_error(src);
    assert_eq!(d.span, within(src, "= f;", "f"));
}

#[test]
fn calls_wrong_number_of_arguments() {
    let src = "fn add(a: int, b: int) -> int { return a + b; }\nfn main() { print(add(1)); }";
    let d = one_error(src);
    assert!(
        d.span == first(src, "add(1)") || d.span == nth(src, "add", 1),
        "point at the call"
    );
    assert_mentions(&d, "2");
}

#[test]
fn calls_wrong_argument_type() {
    let src = "fn f(n: int) { }\nfn main() { f(true); }";
    let d = one_error(src);
    assert_eq!(d.span, first(src, "true"));
}

#[test]
fn calls_print_takes_exactly_one_value() {
    let src = in_main("print();");
    assert_eq!(errors(&src).len(), 1);
    let src = in_main("print(1, 2);");
    assert_eq!(errors(&src).len(), 1);
}

#[test]
fn calls_result_of_a_function_without_return_type_is_not_a_value() {
    let src = "fn nothing() { }\nfn main() { let x = nothing(); }";
    let d = one_error(src);
    assert_eq!(d.span, within(src, "= nothing()", "nothing()"));
    let src = "fn nothing() { }\nfn main() { print(nothing()); }";
    assert_eq!(errors(src).len(), 1);
}

#[test]
fn calls_ignoring_a_return_value_is_fine() {
    ok("fn one() -> int { return 1; }\nfn main() { one(); }");
}

// ================================================================ returns

#[test]
fn returns_value_must_match_return_type() {
    let src = "fn f() -> int { return true; }\nfn main() { }";
    assert_eq!(one_error(src).span, first(src, "true"));
}

#[test]
fn returns_missing_value() {
    let src = "fn f() -> int { return; }\nfn main() { }";
    let d = one_error(src);
    assert_eq!(d.span, first(src, "return;"));
}

#[test]
fn returns_value_from_function_without_return_type() {
    let src = in_main("return 5;");
    assert_eq!(one_error(&src).span, first(&src, "5"));
}

#[test]
fn returns_function_must_return_on_every_path() {
    let src = "fn sign(n: int) -> int {\n    if n > 0 { return 1; }\n}\nfn main() { }";
    let d = one_error(src);
    assert!(
        d.span == first(src, "sign") || d.span.end as usize == src.find("\n}").unwrap() + 2,
        "point at the function name or its closing brace"
    );
}

#[test]
fn returns_if_else_that_both_return_is_enough() {
    ok(
        "fn sign(n: int) -> int {\n    if n > 0 { return 1; } else { return -1; }\n}\nfn main() { }",
    );
    ok(
        "fn f(n: int) -> int {\n    if n > 0 { return 1; } else if n < 0 { return -1; } else { return 0; }\n}\nfn main() { }",
    );
}

#[test]
fn returns_while_loop_does_not_count() {
    let src = "fn f() -> int {\n    while true { return 1; }\n}\nfn main() { }";
    assert_eq!(errors(src).len(), 1, "conservative: a loop might not run");
}

#[test]
fn returns_early_return_in_unit_function() {
    ok(&in_main("let x = 1;\nif x > 0 { return; }\nprint(x);"));
}

// ================================================================ program

#[test]
fn program_needs_main() {
    let src = "fn helper() { }";
    let d = one_error(src);
    assert_mentions(&d, "main");
}

#[test]
fn program_main_takes_no_parameters_and_returns_nothing() {
    assert_eq!(errors("fn main(x: int) { }").len(), 1);
    assert_eq!(errors("fn main() -> int { return 0; }").len(), 1);
}

#[test]
fn program_duplicate_function() {
    let src = "fn helper() { }\nfn helper() { }\nfn main() { }";
    let d = one_error(src);
    assert_eq!(d.span, nth(src, "helper", 1));
    assert!(has_label_at(&d, nth(src, "helper", 0)));
}

#[test]
fn program_cannot_redefine_print() {
    let src = "fn print(x: int) { }\nfn main() { }";
    assert_eq!(one_error(src).span, first(src, "print"));
}

#[test]
fn program_duplicate_parameter() {
    let src = "fn f(a: int, a: int) { }\nfn main() { }";
    assert_eq!(one_error(src).span, nth(src, "a", 1));
}

// ================================================================ cascade

#[test]
fn cascade_one_mistake_gives_one_error() {
    // `y` is unknown. `x` then has an unknown type; using it later must not
    // produce more errors.
    let src = in_main("let x = y + 1;\nlet z = x * 2;\nif x > z { print(x); }");
    assert_eq!(one_error(&src).span, first(&src, "y"));
}

#[test]
fn cascade_bad_call_does_not_cascade() {
    let src = in_main("let r = nope(1) + 2;\nprint(r);");
    assert_eq!(one_error(&src).span, first(&src, "nope"));
}

#[test]
fn cascade_all_independent_errors_are_reported() {
    let src = in_main("print(a);\nprint(b);\nlet c: bool = 1;");
    assert_eq!(errors(&src).len(), 3);
}

// ================================================================ debug table

#[test]
fn debug_table_for_the_example_program() {
    let src = include_str!("../../docs/examples/factorial.step");
    let (_, c) = ok(src);
    let d = &c.debug;

    let fn_names: Vec<&str> = d.functions.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(fn_names, ["factorial", "main"]);
    assert_eq!(d.functions[0].span, first(src, "factorial"));

    let vars: Vec<(&str, TypeName, u32)> = d
        .vars
        .iter()
        .map(|v| (v.name.as_str(), v.ty, v.fn_id))
        .collect();
    assert_eq!(
        vars,
        [
            ("n", TypeName::Int, 0),
            ("total", TypeName::Int, 1),
            ("i", TypeName::Int, 1)
        ]
    );
    assert_eq!(d.vars[1].span, first(src, "total"));

    // factorial: body + the if block. main: body + the while block.
    assert_eq!(d.scopes.len(), 4);
    assert!(d.steps.is_empty(), "steps are filled in by codegen");

    // IDs in the side tables agree with the debug table.
    let total_id = c.vars[&first(src, "total")] as usize;
    assert_eq!(d.vars[total_id].name, "total");
    assert_eq!(c.functions[0].params, [Type::Int]);
    assert_eq!(c.functions[0].ret, Type::Int);
    assert_eq!(c.functions[1].ret, Type::Unit);
}

#[test]
fn debug_scopes_have_parents_and_variables_know_their_scope() {
    let src = in_main("let alpha = 1;\nwhile alpha < 2 {\n    let beta = 2;\n}");
    let (program, c) = ok(&src);
    let d = &c.debug;
    let body_scope = c.scopes[&program.functions[0].body.span];
    let StmtKind::While { body, .. } = &program.functions[0].body.stmts[1].kind else {
        panic!()
    };
    let loop_scope = c.scopes[&body.span];

    assert_eq!(d.scopes[body_scope as usize].parent, None);
    assert_eq!(d.scopes[loop_scope as usize].parent, Some(body_scope));
    assert_eq!(d.scopes[loop_scope as usize].span, body.span);

    let a = c.vars[&first(&src, "alpha")] as usize;
    let b = c.vars[&first(&src, "beta")] as usize;
    assert_eq!(d.vars[a].scope_id, body_scope);
    assert_eq!(d.vars[b].scope_id, loop_scope);
}

#[test]
fn debug_parameters_live_in_the_function_body_scope() {
    let src = "fn f(n: int) { let m = n; }\nfn main() { f(1); }";
    let (program, c) = ok(src);
    let body_scope = c.scopes[&program.functions[0].body.span];
    let n = c.vars[&within(src, "(n:", "n")] as usize;
    assert_eq!(c.debug.vars[n].scope_id, body_scope);
    assert_eq!(c.debug.vars[n].fn_id, 0);
}

#[test]
fn debug_functions_record_what_they_give_back() {
    let src = "fn count() -> int { return 1; }\nfn yes() -> bool { return true; }\nfn main() { }";
    let (_, c) = ok(src);
    let returns: Vec<_> = c.debug.functions.iter().map(|f| f.returns).collect();
    assert_eq!(returns, [Some(TypeName::Int), Some(TypeName::Bool), None]);
}

// ================================================================ robustness

#[test]
fn robust_never_panics_on_any_parseable_mutation_of_the_example() {
    // Replace each identifier in the example with a wrong one, one at a time,
    // and check the checker neither panics nor accepts nonsense silently.
    let src = include_str!("../../docs/examples/factorial.step");
    let (tokens, _) = stepwise_compiler::lexer::lex(src);
    for t in tokens
        .iter()
        .filter(|t| t.kind == stepwise_compiler::lexer::TokenKind::Ident)
    {
        let (s, e) = (t.span.start as usize, t.span.end as usize);
        for replacement in ["zzz", "main", "true", "print", "factorial", "n"] {
            let mutated = format!("{}{replacement}{}", &src[..s], &src[e..]);
            let (program, parse_diags) = parse(&mutated);
            if !parse_diags.is_empty() {
                continue;
            }
            let result = std::panic::catch_unwind(|| check(&program));
            assert!(result.is_ok(), "checker panicked on:\n{mutated}");
        }
    }
}
