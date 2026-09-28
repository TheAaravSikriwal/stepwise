//! Error-message snapshots: every program in tests/errors/ must fail to
//! compile, and the full rendered error text is snapshot-tested with `insta`.
//! That way error quality can't quietly get worse: any change to a message
//! shows up as a diff you have to approve.
//!
//! First run (or after changing messages):
//!   cargo test --test error_messages      # writes .snap.new files and fails
//!   cargo insta review                    # look at each message, accept or reject
//! (`cargo install cargo-insta` once. Without it, rename .snap.new to .snap to accept.)
//!
//! Read every snapshot as a beginner would: is it clear what's wrong and how to fix it?

use stepwise_compiler::compile;

#[test]
fn error_messages() {
    insta::glob!("errors/*.step", |path| {
        let source = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let out = compile(&source);
        assert!(
            out.has_errors(),
            "{name} is in tests/errors/ but compiled without errors"
        );
        let rendered: Vec<String> = out
            .diagnostics
            .iter()
            .map(|d| d.render(&source, &name))
            .collect();
        insta::assert_snapshot!(rendered.join("\n"));
    });
}
