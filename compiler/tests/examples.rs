//! Every example in docs/examples (the playground's gallery) must compile
//! cleanly and run. Only `infinite_loop.step` may stop early, at the event
//! limit, because that's what it demonstrates.
//!
//! Needs the whole compiler. Run with `cargo test --test examples`.

mod harness;

use stepwise_compiler::{Stage, compile_with};

#[test]
fn gallery_examples_compile_and_run() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/examples");
    let mut failures = Vec::new();
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("docs/examples exists") {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "step") {
            continue;
        }
        count += 1;
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let source = std::fs::read_to_string(&path).unwrap();
        let out = compile_with(&source, Stage::Codegen);
        if !out.diagnostics.is_empty() {
            let rendered: Vec<String> = out
                .diagnostics
                .iter()
                .map(|d| d.render(&source, &name))
                .collect();
            failures.push(format!("{name}: compile errors:\n{}", rendered.join("\n")));
            continue;
        }
        let run = harness::run(out.wasm.as_deref().unwrap());
        let may_stop = name == "infinite_loop.step";
        match (&run.error, may_stop) {
            (Some(e), false) => failures.push(format!("{name}: runtime error: {e}")),
            (None, true) => failures.push(format!("{name}: was supposed to hit the event limit")),
            (Some(e), true) if !e.contains("stopped after") => {
                failures.push(format!("{name}: expected the event limit, got: {e}"))
            }
            _ => {}
        }
        if let Err(e) = harness::check_trace(&run.events, &out.debug, run.error.is_none()) {
            failures.push(format!("{name}: inconsistent trace: {e}"));
        }
    }
    assert!(count >= 8, "expected the gallery examples to be found");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
