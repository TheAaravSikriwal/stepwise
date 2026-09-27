//! Deep-nesting tests, in their own file on purpose: if the parser has no
//! depth limit, these overflow the stack, which aborts the whole test binary.
//! Keeping them separate means the rest of the parser tests still report.
//!
//! If this binary dies with "has overflowed its stack", add the depth limit
//! described in docs/specs/parser.md.

use stepwise_compiler::parser::parse;

const DEPTH: usize = 5000;

#[test]
fn deeply_nested_parentheses_are_an_error_not_a_crash() {
    let src = format!(
        "fn main() {{ let x = {}1{}; }}",
        "(".repeat(DEPTH),
        ")".repeat(DEPTH)
    );
    let (_, diags) = parse(&src);
    assert!(!diags.is_empty(), "expected a 'nested too deeply' error");
}

#[test]
fn deeply_nested_blocks_are_an_error_not_a_crash() {
    let src = format!(
        "fn main() {{ {}{} }}",
        "if a { ".repeat(DEPTH),
        "}".repeat(DEPTH)
    );
    let (_, diags) = parse(&src);
    assert!(!diags.is_empty(), "expected a 'nested too deeply' error");
}

#[test]
fn deeply_nested_unary_operators_are_an_error_not_a_crash() {
    let src = format!("fn main() {{ let x = {}true; }}", "!".repeat(DEPTH));
    let (_, diags) = parse(&src);
    assert!(!diags.is_empty(), "expected a 'nested too deeply' error");
}
