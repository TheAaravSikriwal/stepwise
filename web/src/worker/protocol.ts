// Messages between the page and the compile-and-run worker.

import type { CompileMeta } from "../compiler";
import type { Outcome } from "../runtime/run";
import type { TraceData } from "../runtime/trace";

export type Request = { type: "run"; id: number; source: string };

export type Response =
  /** Compilation failed; `meta.diagnostics` has at least one error. */
  | { type: "compile-error"; id: number; meta: CompileMeta }
  /** Compiled and ran. The trace's buffers are transferred, not copied. */
  | { type: "ran"; id: number; meta: CompileMeta; trace: TraceData; outcome: Outcome }
  /** Something went wrong inside Stepwise itself. */
  | { type: "internal-error"; id: number; message: string };
