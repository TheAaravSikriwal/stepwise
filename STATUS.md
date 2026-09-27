# Status

_Last updated: 2026-09-27_

## Where we are

**Debugger UI is built** (call stack, variables with change highlights, output synced to the step,
step buttons, timeline, keyboard shortcuts). Preview it without a compiler or replay engine at
`http://localhost:5173/?demo`. With the real pipeline it switches on automatically once
`createReplay` works.

**Step 0 is done.** A stub `compile()` returns a hand-built WebAssembly module equivalent to
`fn main() { print(42); }`, including the trace calls. It runs:

- natively under wasmtime in `cargo test` (`compiler/tests/phase0.rs`)
- in Node through the real wasm-pack build (`web/tests/pipeline.test.ts`)
- in the browser, from a Web Worker, in both the dev and production builds

Contracts in place: `span.rs`, `diagnostic.rs` (with snapshot-tested rendering),
`debug_table.rs`, `abi.rs` (the trace imports), plus the JS trace buffer, runtime, event limit,
trap messages, and worker protocol.

## Next

- **Your track, in order:**
  1. **Lexer:** `docs/specs/lexer.md`, then make `cargo test --test lexer` pass (29 tests).
  2. **Parser:** `docs/specs/parser.md`, then `cargo test --test parser --test parser_depth`
     (44 + 3 tests). Review the draft AST in `compiler/src/ast.rs` first; it's shared, so any
     change is fine but tell me.
  3. **Checker:** `docs/specs/checker.md`, then `cargo test --test checker` (58 tests). The output
     types in `compiler/src/checked.rs` are a shared draft.
  4. **Codegen:** `docs/specs/codegen.md`, then `cargo test --test codegen` (7 exact traces, the
     steps table, and 22 golden programs in `compiler/tests/programs/`).
  5. After each stage passes, bump `PIPELINE` in `compiler/src/lib.rs` (Lexer Ã¢â€ â€™ Parser Ã¢â€ â€™ Checker Ã¢â€ â€™
     Codegen), run `npm run wasm` in `web/`, and try broken programs in the playground.
  6. **Replay engine** (independent of the compiler, so do it whenever): `docs/specs/replay.md`,
     then `cd web && npx vitest run tests/replay` (18 correctness + 4 speed tests).
  7. When a piece passes, move its step in `.github/workflows/ci.yml` up into the required tests.
- **My track:** the plumbing for MVP and 1.0 is essentially done (gallery, sharing, language reference, How it works). Next: deployment (GitHub Pages, which needs your OK), the social preview image, and issue templates.
- **Note:** `docs/how-it-works.md` describes undo plus snapshots. Update it once you choose your replay design.

Every test file has been checked against a throwaway reference implementation (outside the
repo, deleted afterwards), so if a test fails, the bug is in the code under test, not the test.

## Environment notes

- Smart App Control was blocking rustc's proc-macro DLLs. Turned off on 2026-09-27; fmt, clippy
  and tests all work now.
- GitHub: https://github.com/TheAaravSikriwal/stepwise (public). Git has no credential helper
  configured, so pushes use `git -c credential.helper= -c "credential.helper=!gh auth git-credential" push`,
  or run `gh auth setup-git` once to make plain `git push` work.

## Open questions

- No LICENSE yet (the plan suggests MIT; needs your name as the copyright holder).