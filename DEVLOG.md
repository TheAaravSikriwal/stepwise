# Dev log

A few lines per session: what happened, what was hard. Raw material for the write-up.

## 2026-09-27: Step 0

- Named the project Stepwise (after ruling out anything time/rewind-themed).
- Wrote the build plan (DEVPLAN.md). Found five gaps in the original trace design: parameter values
  weren't recorded, undoing a return had no data to restore, variables had no scope start or end,
  loop conditions weren't steps, and output wasn't tied to steps. The trace ABI now has `declare`
  and `scope_exit`, and loop conditions emit `line`.
- Toolchain fights on Windows: `rustup-init.exe` refused to run because the long temp path made
  Windows use an 8.3 short name (`RUSTUP~1`), and rustup decides what to do from its own
  filename. Then Smart App Control started blocking rustc from loading its own proc-macro DLLs.
- End to end: hand-built module → wasm-bindgen wrapper → Web Worker → page prints `42`.
  The production build works too.

## 2026-09-27: specs, tests and plumbing for everything else

- Wrote a spec and a failing test suite for each core piece: lexer (29), parser (47), checker (58),
  codegen (7 exact traces + 22 golden programs), replay (22), error-message snapshots (15).
- Validated every suite against a throwaway reference implementation outside the repo. It caught
  real test bugs: `first(src, "n")` matching the `n` in `fn`, a miscounted fixture, and a claim
  about unclosed braces that a parser can't actually know.
- The speed tests proved themselves: a snapshot-based replay passes, while replay-from-the-start
  takes 125 s for 5,000 back-steps against a 1 s budget.
- Codegen's trace is fully specified (the exact events per statement), so the Phase 0 stub's
  trace and real codegen's are identical for `print(42)`.
- Built the debugger UI against the `Replay` interface and a canned demo, plus step over / out /
  breakpoints computed from the trace (call depth + line per step), the gallery, share links,
  and the doc pages.
- Gotchas: the Vite watcher sometimes misses a second edit made right after the first; PowerShell
  mangles quotes in native arguments (commit messages now go through a file).
