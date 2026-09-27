// Hand-built traces for replay tests. Each matches what the compiler emits
// for the program in its comment (the same programs as the exact-trace tests
// in compiler/tests/codegen.rs), so these tests don't need the compiler.

import type { DebugTable, Range } from "../../src/compiler";
import { Kind, TraceBuilder, type TraceData } from "../../src/runtime/trace";

type Ev =
  | ["call", number]
  | ["line", number]
  | ["declare", number, number]
  | ["assign", number, number, number]
  | ["scope_exit", number]
  | ["ret", number]
  | ["print", number]
  | ["print_bool", number];

export function traceOf(events: Ev[]): TraceData {
  const t = new TraceBuilder();
  for (const e of events) {
    switch (e[0]) {
      case "call": t.push(Kind.Call, e[1]); break;
      case "line": t.push(Kind.Line, e[1]); break;
      case "declare": t.push(Kind.Declare, e[1], e[2]); break;
      case "assign": t.push(Kind.Assign, e[1], e[2], e[3]); break;
      case "scope_exit": t.push(Kind.ScopeExit, e[1]); break;
      case "ret": t.push(Kind.Ret, e[1]); break;
      case "print": t.push(Kind.Print, e[1], 0); break;
      case "print_bool": t.push(Kind.Print, e[1], 1); break;
    }
  }
  return t.finish();
}

const r = (from = 0, to = 0): Range => ({ from, to });

/** Builds a debug table. vars: [name, fnId, scopeId, type?]; scopes: [fnId, parent]. */
export function debugOf(opts: {
  functions: string[];
  vars: [string, number, number, ("int" | "bool")?][];
  scopes: [number, number | null][];
  steps: number;
}): DebugTable {
  return {
    functions: opts.functions.map((name) => ({ name, span: r() })),
    vars: opts.vars.map(([name, fnId, scopeId, ty]) => ({ name, ty: ty ?? "int", fnId, scopeId, span: r() })),
    scopes: opts.scopes.map(([fnId, parent]) => ({ fnId, parent, span: r() })),
    steps: Array.from({ length: opts.steps }, (_, i) => r(i, i + 1)),
  };
}

/**
 * fn main() {
 *     let x = 1;          // step 0
 *     let mut y = x + 1;  // step 1
 *     y = y * 10;         // step 2
 *     print(y);           // step 3
 * }
 */
export const straightLine = {
  trace: traceOf([
    ["call", 0],
    ["line", 0], ["declare", 0, 1],
    ["line", 1], ["declare", 1, 2],
    ["line", 2], ["assign", 1, 2, 20],
    ["line", 3], ["print", 20],
    ["scope_exit", 0], ["ret", 0],
  ]),
  debug: debugOf({ functions: ["main"], vars: [["x", 0, 0], ["y", 0, 0]], scopes: [[0, null]], steps: 4 }),
};

/**
 * fn add(a: int, b: int) -> int {
 *     return a + b;       // step 0
 * }
 * fn main() {
 *     print(add(2, 3));   // step 1
 * }
 */
export const call = {
  trace: traceOf([
    ["call", 1],
    ["line", 1],
    ["call", 0], ["declare", 0, 2], ["declare", 1, 3],
    ["line", 0], ["scope_exit", 0], ["ret", 5],
    ["print", 5],
    ["scope_exit", 1], ["ret", 0],
  ]),
  debug: debugOf({
    functions: ["add", "main"],
    vars: [["a", 0, 0], ["b", 0, 0]],
    scopes: [[0, null], [1, null]],
    steps: 2,
  }),
};

/**
 * fn main() {
 *     let mut i = 0;      // step 0
 *     while i < 2 {       // step 1 (the condition)
 *         let sq = i * i; // step 2
 *         i = i + 1;      // step 3
 *     }
 * }
 */
export const loop = {
  trace: traceOf([
    ["call", 0],
    ["line", 0], ["declare", 0, 0],
    ["line", 1], ["line", 2], ["declare", 1, 0], ["line", 3], ["assign", 0, 0, 1], ["scope_exit", 1],
    ["line", 1], ["line", 2], ["declare", 1, 1], ["line", 3], ["assign", 0, 1, 2], ["scope_exit", 1],
    ["line", 1],
    ["scope_exit", 0], ["ret", 0],
  ]),
  debug: debugOf({ functions: ["main"], vars: [["i", 0, 0], ["sq", 0, 1]], scopes: [[0, null], [0, 0]], steps: 4 }),
};

/**
 * fn fact(n: int) -> int {
 *     if n <= 1 { return 1; }     // steps 0 (condition), 1 (return 1)
 *     return n * fact(n - 1);     // step 2
 * }
 * fn main() {
 *     let ok = fact(3) == 6;      // step 3
 *     print(ok);                  // step 4
 * }
 */
export const recursion = {
  trace: traceOf([
    ["call", 1],
    ["line", 3],
    ["call", 0], ["declare", 0, 3], ["line", 0], ["line", 2],
    ["call", 0], ["declare", 0, 2], ["line", 0], ["line", 2],
    ["call", 0], ["declare", 0, 1], ["line", 0], ["line", 1], ["scope_exit", 1], ["scope_exit", 0], ["ret", 1],
    ["scope_exit", 0], ["ret", 2],
    ["scope_exit", 0], ["ret", 6],
    ["declare", 1, 1],
    ["line", 4], ["print_bool", 1],
    ["scope_exit", 2], ["ret", 0],
  ]),
  debug: debugOf({
    functions: ["fact", "main"],
    vars: [["n", 0, 0], ["ok", 1, 2, "bool"]],
    scopes: [[0, null], [0, 0], [1, null]],
    steps: 5,
  }),
};

/**
 * A long loop, for performance tests: `i` counts from 0 to `n`.
 *
 * fn main() {
 *     let mut i = 0;          // step 0
 *     while i < n {           // step 1
 *         i = i + 1;          // step 2
 *     }
 *     print(i);               // step 3
 * }
 */
export function longLoop(n: number) {
  const t = new TraceBuilder();
  t.push(Kind.Call, 0);
  t.push(Kind.Line, 0);
  t.push(Kind.Declare, 0, 0);
  for (let i = 0; i < n; i++) {
    t.push(Kind.Line, 1);
    t.push(Kind.Line, 2);
    t.push(Kind.Assign, 0, i, i + 1);
    t.push(Kind.ScopeExit, 1);
  }
  t.push(Kind.Line, 1);
  t.push(Kind.Line, 3);
  t.push(Kind.Print, n, 0);
  t.push(Kind.ScopeExit, 0);
  t.push(Kind.Ret, 0);
  return {
    trace: t.finish(),
    debug: debugOf({ functions: ["main"], vars: [["i", 0, 0]], scopes: [[0, null], [0, 0]], steps: 4 }),
  };
}
