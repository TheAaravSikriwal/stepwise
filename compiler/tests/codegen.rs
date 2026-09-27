//! Codegen tests. Spec: docs/specs/codegen.md
//!
//! Run with `cargo test --test codegen`. Needs a working parser and checker.
//! Start with `trace_*` (small, exact), then `golden_programs`.

mod harness;

use harness::Event::{self, *};
use stepwise_compiler::{DebugTable, Stage, checker, codegen, compile_with, parser};

/// Parses, checks and compiles `src`, which must have no errors.
fn build(src: &str) -> (Vec<u8>, DebugTable) {
    let (program, diags) = parser::parse(src);
    assert!(diags.is_empty(), "test program must parse: {diags:#?}");
    let (checked, diags) = checker::check(&program);
    assert!(diags.is_empty(), "test program must type-check: {diags:#?}");
    let out = codegen::codegen(&program, &checked);
    let mut debug = checked.debug;
    debug.steps = out.steps;
    (out.wasm, debug)
}

/// Compiles and runs `src`, checks trace consistency, returns the events.
fn events(src: &str) -> Vec<Event> {
    let (wasm, debug) = build(src);
    let run = harness::run(&wasm);
    assert_eq!(run.error, None, "program should run without errors");
    if let Err(e) = harness::check_trace(&run.events, &debug, true) {
        panic!("inconsistent trace: {e}\nfull trace: {:#?}", run.events);
    }
    run.events
}

fn print(s: &str) -> Event {
    Print(s.to_string())
}

// ================================================================ exact traces

#[test]
fn trace_let_assign_print() {
    let src = "fn main() {
    let x = 1;
    let mut y = x + 1;
    y = y * 10;
    print(y);
}";
    assert_eq!(
        events(src),
        [
            Call(0),
            Line(0),
            Declare { var: 0, value: 1 },
            Line(1),
            Declare { var: 1, value: 2 },
            Line(2),
            Assign {
                var: 1,
                old: 2,
                new: 20
            },
            Line(3),
            print("20"),
            ScopeExit(0),
            Ret(0),
        ]
    );
}

#[test]
fn trace_call_with_parameters_and_return() {
    // fn ids: add = 0, main = 1. vars: a = 0, b = 1. scopes: add = 0, main = 1.
    // steps in source order: 0 = `return a + b;`, 1 = `print(add(2, 3));`
    let src = "fn add(a: int, b: int) -> int {
    return a + b;
}

fn main() {
    print(add(2, 3));
}";
    assert_eq!(
        events(src),
        [
            Call(1),
            Line(1),
            Call(0),
            Declare { var: 0, value: 2 },
            Declare { var: 1, value: 3 },
            Line(0),
            ScopeExit(0),
            Ret(5),
            print("5"),
            ScopeExit(1),
            Ret(0),
        ]
    );
}

#[test]
fn trace_while_loop_checks_condition_every_time_and_exits_scope_each_pass() {
    // vars: i = 0, sq = 1. scopes: body = 0, loop = 1.
    // steps: 0 `let mut i`, 1 `i < 2`, 2 `let sq`, 3 `i = i + 1;`
    let src = "fn main() {
    let mut i = 0;
    while i < 2 {
        let sq = i * i;
        i = i + 1;
    }
}";
    assert_eq!(
        events(src),
        [
            Call(0),
            Line(0),
            Declare { var: 0, value: 0 },
            // pass 1
            Line(1),
            Line(2),
            Declare { var: 1, value: 0 },
            Line(3),
            Assign {
                var: 0,
                old: 0,
                new: 1
            },
            ScopeExit(1),
            // pass 2
            Line(1),
            Line(2),
            Declare { var: 1, value: 1 },
            Line(3),
            Assign {
                var: 0,
                old: 1,
                new: 2
            },
            ScopeExit(1),
            // final check fails
            Line(1),
            ScopeExit(0),
            Ret(0),
        ]
    );
}

#[test]
fn trace_if_else_runs_one_branch_and_exits_its_scope() {
    // scopes: body = 0, then = 1, else = 2. steps: 0 `1 > 2`, 1 `print(1);`, 2 `print(2);`
    let src = "fn main() {
    if 1 > 2 {
        print(1);
    } else {
        print(2);
    }
}";
    assert_eq!(
        events(src),
        [
            Call(0),
            Line(0),
            Line(2),
            print("2"),
            ScopeExit(2),
            ScopeExit(0),
            Ret(0)
        ]
    );
}

#[test]
fn trace_return_from_inside_nested_blocks_exits_every_scope() {
    // fn ids: find = 0, main = 1. vars: n = 0, r = 1.
    // scopes: find body 0, while body 1, if block 2, main body 3.
    // steps: 0 `let mut n`, 1 `true`, 2 `n == 1`, 3 `return n;`,
    //        4 `n = n + 1;`, 5 `return -1;`, 6 `let r`
    let src = "fn find() -> int {
    let mut n = 0;
    while true {
        if n == 1 {
            return n;
        }
        n = n + 1;
    }
    return -1;
}

fn main() {
    let r = find();
}";
    assert_eq!(
        events(src),
        [
            Call(1),
            Line(6),
            Call(0),
            Line(0),
            Declare { var: 0, value: 0 },
            // pass 1: the condition is false, so no if-block events
            Line(1),
            Line(2),
            Line(4),
            Assign {
                var: 0,
                old: 0,
                new: 1
            },
            ScopeExit(1),
            // pass 2: return from inside if, inside while, inside the body
            Line(1),
            Line(2),
            Line(3),
            ScopeExit(2),
            ScopeExit(1),
            ScopeExit(0),
            Ret(1),
            Declare { var: 1, value: 1 },
            ScopeExit(3),
            Ret(0),
        ]
    );
}

#[test]
fn trace_early_return_in_unit_function() {
    // steps: 0 `true`, 1 `return;`, 2 `print(1);`
    let src = "fn main() {
    if true {
        return;
    }
    print(1);
}";
    assert_eq!(
        events(src),
        [
            Call(0),
            Line(0),
            Line(1),
            ScopeExit(1),
            ScopeExit(0),
            Ret(0)
        ]
    );
}

#[test]
fn trace_print_bool() {
    let src = "fn main() {\n    print(3 < 4);\n}";
    assert_eq!(
        events(src),
        [Call(0), Line(0), print("true"), ScopeExit(0), Ret(0)]
    );
}

// ================================================================ steps

#[test]
fn steps_highlight_statements_and_conditions() {
    let src = include_str!("../../docs/examples/factorial.step");
    let (_, debug) = build(src);
    let texts: Vec<&str> = debug.steps.iter().map(|s| s.text(src)).collect();
    assert_eq!(
        texts,
        [
            "n <= 1",
            "return 1;",
            "return n * factorial(n - 1);",
            "let mut total = 0;",
            "let mut i = 1;",
            "i <= 5",
            "total = total + factorial(i);",
            "i = i + 1;",
            "print(total);",
        ]
    );
}

#[test]
fn full_pipeline_fills_the_debug_table() {
    let src = include_str!("../../docs/examples/factorial.step");
    let out = compile_with(src, Stage::Codegen);
    assert!(out.diagnostics.is_empty());
    assert!(out.wasm.is_some());
    assert_eq!(out.debug.steps.len(), 9);
    assert_eq!(out.debug.functions.len(), 2);
}

// ================================================================ golden programs

struct Golden {
    name: String,
    source: String,
    output: Vec<String>,
    trap: Option<String>,
}

fn load_golden_programs() -> Vec<Golden> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");
    let mut programs: Vec<Golden> = std::fs::read_dir(&dir)
        .expect("tests/programs exists")
        .map(|entry| {
            let path = entry.unwrap().path();
            let source = std::fs::read_to_string(&path)
                .unwrap()
                .replace("\r\n", "\n");
            let mut output = Vec::new();
            let mut trap = None;
            for line in source.lines() {
                if let Some(rest) = line.strip_prefix("// expect: ") {
                    output.push(rest.to_string());
                } else if let Some(rest) = line.strip_prefix("// expect-trap: ") {
                    trap = Some(rest.to_string());
                }
            }
            Golden {
                name: path.file_name().unwrap().to_string_lossy().into_owned(),
                source,
                output,
                trap,
            }
        })
        .collect();
    programs.sort_by(|a, b| a.name.cmp(&b.name));
    programs
}

#[test]
fn golden_programs() {
    let programs = load_golden_programs();
    assert!(
        programs.len() >= 20,
        "expected the golden programs to be found"
    );
    let mut failures = Vec::new();

    for g in &programs {
        let result = std::panic::catch_unwind(|| {
            let (wasm, debug) = build(&g.source);
            (harness::run(&wasm), debug)
        });
        let (run, debug) = match result {
            Ok(r) => r,
            Err(_) => {
                failures.push(format!("{}: panicked while compiling or running", g.name));
                continue;
            }
        };
        if run.output != g.output {
            failures.push(format!(
                "{}: wrong output\n    expected: {:?}\n    actual:   {:?}",
                g.name, g.output, run.output
            ));
        }
        match (&g.trap, &run.error) {
            (None, Some(e)) => failures.push(format!("{}: unexpected runtime error: {e}", g.name)),
            (Some(t), None) => failures.push(format!(
                "{}: expected a runtime error containing {t:?}",
                g.name
            )),
            (Some(t), Some(e)) if !e.contains(t.as_str()) => failures.push(format!(
                "{}: expected an error containing {t:?}, got: {e}",
                g.name
            )),
            _ => {}
        }
        if let Err(e) = harness::check_trace(&run.events, &debug, run.error.is_none()) {
            failures.push(format!("{}: inconsistent trace: {e}", g.name));
        }
    }

    if !failures.is_empty() {
        panic!(
            "{} of {} golden programs failed:\n\n{}",
            failures.len(),
            programs.len(),
            failures.join("\n\n")
        );
    }
}
