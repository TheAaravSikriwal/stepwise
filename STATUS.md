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

- **My track:** Step 1a. Lexer spec (`docs/specs/lexer.md`), token types, failing lexer tests,
  and the `stepc` dev CLI. Then 1b, the CodeMirror editor.
- **Your track:** read *Crafting Interpreters*, chapter 4 ("Scanning"), then the lexer spec once
  it's written.

## Open issues

- **Smart App Control is blocking Rust builds.** It intermittently blocks `rustc` from loading
  proc-macro DLLs it just built (Code Integrity event 3077), and blocks `cargo-fmt.exe` entirely.
  `cargo clippy` fails every time; `cargo test` sometimes does. Waiting on your decision about how
  to handle it.
- No GitHub remote yet. Creating it and deploying are public actions, so ask first.
- No LICENSE yet (the plan suggests MIT; needs your name as the copyright holder).
