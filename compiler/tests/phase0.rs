//! Phase 0 exit check: the stub module is valid, runs, and emits the trace
//! events real codegen will emit. Delete when `stub.rs` goes away.

mod harness;

use harness::Event;

#[test]
fn stub_prints_42_with_trace_events() {
    let out = stepwise_compiler::compile("fn main() { print(42); }");
    assert!(out.diagnostics.is_empty());
    let run = harness::run(out.wasm.as_deref().expect("stub always produces wasm"));

    assert_eq!(run.error, None);
    assert_eq!(run.output, ["42"]);
    assert_eq!(
        run.events,
        [
            Event::Call(0),
            Event::Line(0),
            Event::Print("42".into()),
            Event::ScopeExit(0),
            Event::Ret(0),
        ]
    );
}
