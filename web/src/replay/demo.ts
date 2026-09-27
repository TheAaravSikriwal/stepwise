// A canned debugging session for previewing the debugger UI before the real
// replay engine and compiler exist. Open the playground with `?demo`.
// The states are written out by hand; they match the `loop` fixture in
// tests/fixtures/traces.ts.

import type { DebugTable, Range } from "../compiler";
import { StepIndex } from "../ui/steps";
import type { Replay, ReplayState } from "./types";

export const DEMO_SOURCE = `fn main() {
    let mut i = 0;
    while i < 2 {
        let sq = i * i;
        i = i + 1;
    }
}
`;

function span(text: string, from = 0): Range {
  const start = DEMO_SOURCE.indexOf(text, from);
  return { from: start, to: start + text.length };
}

/** The whole block starting at the first `{` at or after `from`, as the compiler records scopes. */
function block(from = 0): Range {
  const start = DEMO_SOURCE.indexOf("{", from);
  let depth = 0;
  for (let i = start; i < DEMO_SOURCE.length; i++) {
    if (DEMO_SOURCE[i] === "{") depth++;
    if (DEMO_SOURCE[i] === "}" && --depth === 0) return { from: start, to: i + 1 };
  }
  return { from: start, to: DEMO_SOURCE.length };
}

export const DEMO_DEBUG: DebugTable = {
  functions: [{ name: "main", span: span("main") }],
  vars: [
    { name: "i", ty: "int", fnId: 0, scopeId: 0, span: span("i =") },
    { name: "sq", ty: "int", fnId: 0, scopeId: 1, span: span("sq") },
  ],
  scopes: [
    { fnId: 0, parent: null, span: block() },
    { fnId: 0, parent: 0, span: block(DEMO_SOURCE.indexOf("while")) },
  ],
  steps: [span("let mut i = 0;"), span("i < 2"), span("let sq = i * i;"), span("i = i + 1;")],
};

// [line, i, sq, changed]: `undefined` means "not declared".
const rows: [number | null, number | undefined, number | undefined, string[]][] = [
  [0, undefined, undefined, []],
  [1, 0, undefined, ["0:0"]],
  [2, 0, undefined, []],
  [3, 0, 0, ["0:1"]],
  [1, 1, undefined, ["0:0"]],
  [2, 1, undefined, []],
  [3, 1, 1, ["0:1"]],
  [1, 2, undefined, ["0:0"]],
  [null, undefined, undefined, []],
];

function stateAt(step: number): ReplayState {
  const [line, i, sq, changed] = rows[step];
  const vars = [];
  if (i !== undefined) vars.push({ varId: 0, name: "i", value: i, display: String(i) });
  if (sq !== undefined) vars.push({ varId: 1, name: "sq", value: sq, display: String(sq) });
  return {
    step,
    line,
    frames: line === null ? [] : [{ callId: 0, fnId: 0, name: "main", line, vars }],
    output: [],
    changed: new Set(changed),
  };
}

export function createDemoReplay(): Replay {
  let step = 0;
  return {
    stepCount: rows.length,
    seek: (k) => void (step = Math.max(0, Math.min(rows.length - 1, Math.trunc(k)))),
    state: () => stateAt(step),
  };
}

/** Step navigation for the demo: everything happens in `main`. */
export function createDemoStepIndex(lineOf: (offset: number) => number): StepIndex {
  const depth = Int32Array.from(rows, ([line]) => (line === null ? 0 : 1));
  const lines = Int32Array.from(rows, ([line]) => (line === null ? 0 : lineOf(DEMO_DEBUG.steps[line].from)));
  return new StepIndex(depth, lines);
}
