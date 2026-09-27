# Status

_Last updated: 2026-09-27_

## Where we are

**Step 0 is done.** A stub `compile()` returns a hand-built WebAssembly module equivalent to
`fn main() { print(42); }`, including the trace calls. It runs:

- natively under wasmtime in `cargo test` (`compiler/tests/phase0.rs`)
- in Node through the real wasm-pack build (`web/tests/pipeline.test.ts`)
- in the browser, from a Web Worker, in both the dev and production builds

Contracts in place: `span.rs`, `diagnostic.rs` (with snapshot-tested rendering),
`debug_table.rs`, `abi.rs` (the trace imports), plus the JS trace buffer, runtime, event limit,
trap messages, and worker protocol.

## Next

- **Your track:** the lexer. Read `docs/specs/lexer.md`, then make `cargo test --test lexer` pass
  (29 tests, currently failing at `todo!()` as intended). *Crafting Interpreters*, chapter 4, is
  the background reading. When they pass, move the lexer step in `.github/workflows/ci.yml` up
  into the required tests.
- **My track:** Step 1b, the CodeMirror editor: syntax highlighting, error underlines,
  Ctrl+Enter to run.

## Environment notes

- Smart App Control was blocking rustc's proc-macro DLLs. Turned off on 2026-09-27; fmt, clippy
  and tests all work now.
- GitHub: https://github.com/TheAaravSikriwal/stepwise (public). Git has no credential helper
  configured, so pushes use `git -c credential.helper= -c "credential.helper=!gh auth git-credential" push`,
  or run `gh auth setup-git` once to make plain `git push` work.

## Open questions

- No LICENSE yet (the plan suggests MIT; needs your name as the copyright holder).