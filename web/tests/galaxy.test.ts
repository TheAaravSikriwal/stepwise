import { describe, expect, it } from "vitest";
import { buildCallTree, eventsThroughStep, layout, phasesAt } from "../src/galaxy/model";
import { call, longLoop, recursion, straightLine } from "./fixtures/traces";

// recursion fixture: main → fact(3) → fact(2) → fact(1), steps 0..8 (8 = finished).
describe("the call tree", () => {
  const tree = buildCallTree(recursion.trace, recursion.debug);

  it("has one node per call, nested as they were made", () => {
    expect(tree.nodes.map((n) => `${n.name}@${n.depth}`)).toEqual(["main@0", "fact@1", "fact@2", "fact@3"]);
    expect(tree.nodes.map((n) => n.parent)).toEqual([-1, 0, 1, 2]);
    expect(tree.nodes[1].children).toEqual([2]);
    expect(tree.hiddenTotal).toBe(0);
  });

  it("knows how long each call ran itself, and where it starts", () => {
    // main: steps 0 and 7; each fact: its own condition + return lines.
    expect(tree.nodes.map((n) => n.ownSteps)).toEqual([2, 2, 2, 2]);
    expect(tree.nodes.map((n) => n.firstStep)).toEqual([0, 1, 3, 5]);
  });

  it("gives each call's phase at any step, and the current call", () => {
    const at = (step: number) => phasesAt(tree, eventsThroughStep(recursion.trace, step));
    expect(at(0)).toEqual({ phases: ["running", "waiting", "waiting", "waiting"], current: 0 });
    expect(at(6)).toEqual({ phases: ["running", "running", "running", "running"], current: 3 });
    expect(at(7)).toEqual({ phases: ["running", "done", "done", "done"], current: 0 });
    expect(at(8).phases).toEqual(["done", "done", "done", "done"]);
    expect(at(8).current).toBe(-1);
  });

  it("handles a call per statement, and code with no calls", () => {
    expect(buildCallTree(call.trace, call.debug).nodes.map((n) => n.name)).toEqual(["main", "add"]);
    expect(buildCallTree(straightLine.trace, straightLine.debug).nodes).toHaveLength(1);
  });

  it("folds calls past the cap into their ancestor, keeping the top of the tree", () => {
    const capped = buildCallTree(recursion.trace, recursion.debug, 2);
    expect(capped.nodes.map((n) => n.name)).toEqual(["main", "fact"]);
    expect(capped.nodes[1].hidden).toBe(2);
    expect(capped.hiddenTotal).toBe(2);
  });

  it("builds quickly on a long run", () => {
    const big = longLoop(250_000);
    const start = performance.now();
    buildCallTree(big.trace, big.debug);
    expect(performance.now() - start).toBeLessThan(500);
  });
});

describe("the 3D layout", () => {
  const tree = buildCallTree(recursion.trace, recursion.debug);
  const pos = layout(tree);

  it("puts main at the centre and every call somewhere real and distinct", () => {
    expect(pos[0]).toEqual([0, 0, 0]);
    for (const p of pos) for (const x of p) expect(Number.isFinite(x)).toBe(true);
    const keys = new Set(pos.map((p) => p.map((x) => x.toFixed(3)).join(",")));
    expect(keys.size).toBe(pos.length);
  });

  it("is the same every time, so bubbles never jump while scrubbing", () => {
    expect(layout(buildCallTree(recursion.trace, recursion.debug))).toEqual(pos);
  });
});
