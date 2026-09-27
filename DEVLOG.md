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
