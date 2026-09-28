// Runs a compiled module, recording every host call into a trace.
//
// The program always runs to completion (or until it stops) before anything
// is shown. Stepping and rewinding happen later, over the recording.

import { Kind, TraceBuilder, type TraceData } from "./trace";

export const DEFAULT_EVENT_LIMIT = 1_000_000;

export type Outcome =
  | { kind: "ok" }
  /** Hit the event limit: probably an infinite loop. */
  | { kind: "limit"; limit: number }
  /** A runtime error, already in plain English. */
  | { kind: "error"; message: string };

export interface RunResult {
  trace: TraceData;
  outcome: Outcome;
}

class EventLimitReached extends Error {}

export async function run(wasm: Uint8Array, eventLimit = DEFAULT_EVENT_LIMIT): Promise<RunResult> {
  const trace = new TraceBuilder();

  const record = (kind: Kind, a = 0, b = 0, c = 0) => {
    if (trace.length >= eventLimit) throw new EventLimitReached();
    trace.push(kind, a, b, c);
  };

  // Must match compiler/src/abi.rs.
  const sw = {
    line: (span: number) => record(Kind.Line, span),
    declare: (v: number, value: number) => record(Kind.Declare, v, value),
    assign: (v: number, old: number, value: number) => record(Kind.Assign, v, old, value),
    call: (fn: number) => record(Kind.Call, fn),
    ret: (value: number) => record(Kind.Ret, value),
    scope_exit: (scope: number) => record(Kind.ScopeExit, scope),
    print: (value: number) => record(Kind.Print, value, 0),
    print_bool: (value: number) => record(Kind.Print, value, 1),
  };

  const { instance } = await WebAssembly.instantiate(wasm as BufferSource, { sw });
  const main = instance.exports.main as () => void;

  let outcome: Outcome;
  try {
    main();
    outcome = { kind: "ok" };
  } catch (e) {
    outcome =
      e instanceof EventLimitReached
        ? { kind: "limit", limit: eventLimit }
        : { kind: "error", message: explainError(e) };
  }
  return { trace: trace.finish(), outcome };
}

/** Turns a WebAssembly trap or engine error into a beginner-friendly sentence. */
export function explainError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  // Messages differ between browsers, so match loosely.
  if (/divide by zero|division by zero/i.test(msg)) return "Division by zero.";
  if (/integer overflow/i.test(msg))
    return "Integer overflow: the result doesn't fit in an int (for example, the smallest int divided by -1).";
  if (/call stack|too much recursion|stack overflow/i.test(msg))
    return "Recursion went too deep. Is there a base case that stops it?";
  if (/unreachable/i.test(msg))
    return "Reached a point the compiler thought was impossible. This is a Stepwise bug; please report it.";
  return `Runtime error: ${msg}`;
}

export function describeOutcome(outcome: Outcome): string | null {
  switch (outcome.kind) {
    case "ok":
      return null;
    case "limit":
      return "It ran for a very long time, so Stepwise stopped it. Is there a loop that never ends? You can still step through everything that ran.";
    case "error":
      return outcome.message;
  }
}
