// Replay engine tests. Spec: docs/specs/replay.md
// Run with `npx vitest run tests/replay`.

import { describe, expect, it } from "vitest";
import type { DebugTable } from "../src/compiler";
import { createReplay } from "../src/replay/replay";
import type { Replay, ReplayState } from "../src/replay/types";
import type { TraceData } from "../src/runtime/trace";
import { call, loop, recursion, straightLine } from "./fixtures/traces";

/** A compact, comparable view of a state: frames as `name@line: var=value ...`. */
function show(s: ReplayState) {
  return {
    line: s.line,
    frames: s.frames.map(
      (f) => `${f.name}@${f.line}:` + f.vars.map((v) => ` ${v.name}=${v.display}`).join(""),
    ),
    output: s.output,
    changed: [...s.changed].sort(),
  };
}

function at(replay: Replay, step: number) {
  replay.seek(step);
  return show(replay.state());
}

function fresh(f: { trace: TraceData; debug: DebugTable }) {
  return createReplay(f.trace, f.debug);
}

const fixtures = { straightLine, call, loop, recursion };

describe("step counts", () => {
  it("has one step per line event, plus a final step", () => {
    expect(fresh(straightLine).stepCount).toBe(5);
    expect(fresh(call).stepCount).toBe(3);
    expect(fresh(loop).stepCount).toBe(9);
    expect(fresh(recursion).stepCount).toBe(9);
  });

  it("starts at step 0", () => {
    const r = fresh(straightLine);
    expect(r.state().step).toBe(0);
  });

  it("clamps out-of-range seeks", () => {
    const r = fresh(straightLine);
    r.seek(-5);
    expect(r.state().step).toBe(0);
    r.seek(999);
    expect(r.state().step).toBe(4);
  });
});

describe("states", () => {
  it("straight-line code", () => {
    const r = fresh(straightLine);
    expect(at(r, 0)).toEqual({ line: 0, frames: ["main@0:"], output: [], changed: [] });
    expect(at(r, 1)).toEqual({ line: 1, frames: ["main@1: x=1"], output: [], changed: ["0:0"] });
    expect(at(r, 2)).toEqual({ line: 2, frames: ["main@2: x=1 y=2"], output: [], changed: ["0:1"] });
    expect(at(r, 3)).toEqual({ line: 3, frames: ["main@3: x=1 y=20"], output: [], changed: ["0:1"] });
    // The final step: the program finished, output complete, nothing on the stack.
    expect(at(r, 4)).toEqual({ line: null, frames: [], output: ["20"], changed: [] });
  });

  it("a call pushes a frame with its parameters, and the caller keeps its line", () => {
    const r = fresh(call);
    expect(at(r, 0)).toEqual({ line: 1, frames: ["main@1:"], output: [], changed: [] });
    expect(at(r, 1)).toEqual({
      line: 0,
      frames: ["main@1:", "add@0: a=2 b=3"],
      output: [],
      changed: ["1:0", "1:1"],
    });
    expect(at(r, 2)).toEqual({ line: null, frames: [], output: ["5"], changed: [] });
  });

  it("loop variables leave scope at the end of each pass", () => {
    const r = fresh(loop);
    const lines = Array.from({ length: 9 }, (_, i) => at(r, i));
    expect(lines.map((s) => s.frames[0] ?? "(done)")).toEqual([
      "main@0:",
      "main@1: i=0",
      "main@2: i=0",
      "main@3: i=0 sq=0",
      "main@1: i=1", // `sq` is gone: its scope ended
      "main@2: i=1",
      "main@3: i=1 sq=1",
      "main@1: i=2",
      "(done)",
    ]);
    expect(lines.map((s) => s.changed)).toEqual([[], ["0:0"], [], ["0:1"], ["0:0"], [], ["0:1"], ["0:0"], []]);
  });

  it("recursion shows every frame, each with its own `n`", () => {
    const r = fresh(recursion);
    // Step 6: about to `return 1;` in the innermost call.
    expect(at(r, 6).frames).toEqual(["main@3:", "fact@2: n=3", "fact@2: n=2", "fact@1: n=1"]);
    expect(at(r, 6).line).toBe(1);
    // After all the returns: back in main, `ok` declared, about to print.
    expect(at(r, 7)).toEqual({ line: 4, frames: ["main@4: ok=true"], output: [], changed: ["0:1"] });
    expect(at(r, 8)).toEqual({ line: null, frames: [], output: ["true"], changed: [] });
  });

  it("bools display as true/false", () => {
    const r = fresh(recursion);
    r.seek(7);
    const ok = r.state().frames[0].vars[0];
    expect(ok).toEqual({ varId: 1, name: "ok", value: 1, display: "true" });
  });
});

describe("output is synced to time", () => {
  it("printed lines disappear when stepping back before the print", () => {
    const r = fresh(straightLine);
    expect(at(r, 4).output).toEqual(["20"]);
    expect(at(r, 3).output).toEqual([]);
    expect(at(r, 4).output).toEqual(["20"]);
  });
});

describe("the key property: a step's state doesn't depend on how you got there", () => {
  for (const [name, f] of Object.entries(fixtures)) {
    it(`${name}: stepping backward matches seeking directly`, () => {
      const n = fresh(f).stepCount;
      const direct = Array.from({ length: n }, (_, i) => at(fresh(f), i));

      const r = fresh(f);
      for (let i = 0; i < n; i++) expect(at(r, i), `forward to ${i}`).toEqual(direct[i]);
      for (let i = n - 1; i >= 0; i--) expect(at(r, i), `backward to ${i}`).toEqual(direct[i]);
    });

    it(`${name}: random jumps match seeking directly`, () => {
      const n = fresh(f).stepCount;
      const direct = Array.from({ length: n }, (_, i) => at(fresh(f), i));
      const r = fresh(f);
      let seed = 12345;
      for (let k = 0; k < 200; k++) {
        seed = (seed * 1103515245 + 12345) % 2 ** 31;
        const target = seed % n;
        expect(at(r, target), `jump ${k} to ${target}`).toEqual(direct[target]);
      }
    });
  }

  it("state() returns a snapshot that later seeks don't change", () => {
    const r = fresh(straightLine);
    r.seek(3);
    const s3 = r.state();
    const before = show(s3);
    r.seek(0);
    r.seek(4);
    expect(show(s3)).toEqual(before);
  });
});
