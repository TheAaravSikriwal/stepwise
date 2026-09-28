import { describe, expect, it } from "vitest";
import type { DebugTable } from "../src/compiler";
import { DEMO_DEBUG, DEMO_SOURCE, createDemoReplay } from "../src/replay/demo";
import type { ReplayState } from "../src/replay/types";
import { narrate } from "../src/ui/narrate";

const lineOf = (source: string) => (offset: number) => source.slice(0, offset).split("\n").length;

/** Narration for every step of the demo program (a small while loop). */
function demoNarration() {
  const r = createDemoReplay();
  const ctx = { debug: DEMO_DEBUG, source: DEMO_SOURCE, lineOf: lineOf(DEMO_SOURCE) };
  const states: ReplayState[] = [];
  for (let k = 0; k < r.stepCount; k++) {
    r.seek(k);
    states.push(r.state());
  }
  return states.map((s, k) => narrate(k === 0 ? null : states[k - 1], s, ctx));
}

describe("narrate: the demo loop, step by step", () => {
  const n = demoNarration();

  it("starts by saying nothing has run, and where it begins", () => {
    expect(n[0].happened).toEqual(["Your program is ready to start. Nothing has run yet."]);
    expect(n[0].next).toBe("Next up: line 2, highlighted in yellow.");
  });

  it("names new variables with their values", () => {
    expect(n[1].happened).toEqual(["Made a new variable `i` and set it to 0."]);
    expect(n[3].happened).toEqual(["Made a new variable `sq` and set it to 0."]);
  });

  it("says what a loop condition decided", () => {
    expect(n[2].happened).toEqual(["Checked `i < 2`: it's true, so the loop runs its body."]);
    expect(n[8].happened[0]).toBe("Checked `i < 2`: it's false, so the loop is done.");
  });

  it("gives old and new values, and says when a variable goes away", () => {
    expect(n[4].happened).toEqual([
      "`i` changed from 0 to 1.",
      "`sq` went away, because the block it was made in ended.",
    ]);
  });

  it("ends by saying the program finished, with nothing next", () => {
    expect(n[8].happened.at(-1)).toBe("The program finished.");
    expect(n[8].next).toBeNull();
  });
});

describe("narrate: calls, returns, prints and errors", () => {
  const source = "fn add(a: int, b: int) -> int {\n    return a + b;\n}\nfn main() {\n    print(add(2, 3));\n}\n";
  const at = (text: string) => {
    const from = source.indexOf(text);
    return { from, to: from + text.length };
  };
  const debug: DebugTable = {
    functions: [
      { name: "add", span: at("add") },
      { name: "main", span: at("main") },
    ],
    vars: [
      { name: "a", ty: "int", fnId: 0, scopeId: 0, span: at("a:") },
      { name: "b", ty: "int", fnId: 0, scopeId: 0, span: at("b:") },
    ],
    scopes: [
      { fnId: 0, parent: null, span: at("{\n    return") },
      { fnId: 1, parent: null, span: at("{\n    print") },
    ],
    steps: [at("return a + b;"), at("print(add(2, 3));")],
  };
  const ctx = { debug, source, lineOf: lineOf(source) };
  const main = (line: number | null) => ({ callId: 0, fnId: 1, name: "main", line, vars: [] });
  const add = {
    callId: 2,
    fnId: 0,
    name: "add",
    line: 0,
    vars: [
      { varId: 0, name: "a", value: 2, display: "2" },
      { varId: 1, name: "b", value: 3, display: "3" },
    ],
  };
  const state = (s: Partial<ReplayState>): ReplayState => ({
    step: 0,
    line: null,
    frames: [],
    output: [],
    changed: new Set(),
    ...s,
  });
  const inMain = state({ step: 0, line: 1, frames: [main(1)] });
  const inAdd = state({ step: 1, line: 0, frames: [main(1), add] });
  const done = state({ step: 2, line: null, frames: [], output: ["5"] });

  it("says which function was called, with its arguments, and where we are now", () => {
    const n = narrate(inMain, inAdd, ctx);
    expect(n.happened).toEqual(["Called `add` with a = 2, b = 3."]);
    expect(n.next).toBe("Next up: line 2 in `add`, highlighted in yellow.");
  });

  it("says a function finished, and what was printed", () => {
    const n = narrate(inAdd, done, ctx);
    expect(n.happened).toEqual([
      "`add` finished and went back to `main`.",
      "Printed 5.",
      "The program finished.",
    ]);
  });

  it("explains a runtime error at the end, with the line it stopped on", () => {
    const n = narrate(inAdd, state({ step: 2, frames: [] }), ctx, "Division by zero.");
    expect(n.happened.at(-1)).toBe("The program stopped on line 2: Division by zero.");
    expect(n.happened.some((h) => h.startsWith("Ran line"))).toBe(false);
  });

  it("doesn't read a condition's result into a run that was stopped", () => {
    // The last step was checking a while condition when the step limit hit.
    const cond = { ...inMain, line: 1 };
    const n = narrate(cond, state({ step: 2, frames: [] }), ctx, "Stopped after 1,000,000 steps.");
    expect(n.happened.some((h) => h.startsWith("Checked"))).toBe(false);
  });

  it("tells a new call apart from the one it replaced, at the same depth", () => {
    // `add(1, 1) + add(2, 3)`: one call finishes and the next starts within a
    // single step, both at depth 2. That's a return and a call, not `a`
    // and `b` changing.
    const first = { ...add, callId: 2, vars: [{ varId: 0, name: "a", value: 1, display: "1" }, { varId: 1, name: "b", value: 1, display: "1" }] };
    const second = { ...add, callId: 9 };
    const n = narrate(state({ line: 0, frames: [main(1), first] }), state({ line: 0, frames: [main(1), second] }), ctx);
    expect(n.happened).toEqual(["`add` finished and went back to `main`.", "Called `add` with a = 2, b = 3."]);
  });
});