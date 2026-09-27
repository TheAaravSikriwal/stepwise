// The replay engine: turns a recorded trace into the program's state at any
// step, forward or backward.
//
// **This file is yours to write.** Spec: docs/specs/replay.md
// Tests: web/tests/replay.test.ts and replay-perf.test.ts
// (run with `npx vitest run tests/replay`).

import type { DebugTable } from "../compiler";
import type { TraceData } from "../runtime/trace";
import type { Replay } from "./types";

export function createReplay(trace: TraceData, debug: DebugTable): Replay {
  void trace;
  void debug;
  throw new Error("createReplay is not written yet: see docs/specs/replay.md");
}
