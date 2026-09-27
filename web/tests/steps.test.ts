import { describe, expect, it } from "vitest";
import { StepIndex } from "../src/ui/steps";
import { recursion, straightLine } from "./fixtures/traces";

// In the fixtures, step span i covers offsets i..i+1, so call it line i + 1.
const lineOf = (offset: number) => offset + 1;

// recursion fixture, step by step (depth: 1 = main):
//   0 main `let ok = fact(3) == 6;`  depth 1, line 4
//   1 fact(3) `n <= 1`              depth 2, line 1
//   2 fact(3) `return n * fact(..)` depth 2, line 3
//   3 fact(2) `n <= 1`              depth 3, line 1
//   4 fact(2) `return n * fact(..)` depth 3, line 3
//   5 fact(1) `n <= 1`              depth 4, line 1
//   6 fact(1) `return 1;`           depth 4, line 2
//   7 main `print(ok);`             depth 1, line 5
//   8 finished
const index = StepIndex.fromTrace(recursion.trace, recursion.debug, lineOf);

describe("StepIndex", () => {
  it("knows each step's line", () => {
    expect([0, 1, 2, 3, 4, 5, 6, 7, 8].map((s) => index.lineAt(s))).toEqual([4, 1, 3, 1, 3, 1, 2, 5, 0]);
  });

  it("step over skips the whole call", () => {
    expect(index.nextOver(0)).toBe(7);
    expect(index.prevOver(7)).toBe(0);
  });

  it("step over at a return goes to the caller", () => {
    expect(index.nextOver(6)).toBe(7);
  });

  it("step out finishes the function, and back out returns to the call site", () => {
    expect(index.nextOut(4)).toBe(7); // fact(3) has no statements left after the call
    expect(index.prevOut(4)).toBe(2); // the `return n * fact(n - 1)` that called fact(2)
    expect(index.prevOut(0)).toBe(0); // nowhere to go from main
  });

  it("breakpoints find the next and previous step on a line", () => {
    const onLine3 = new Set([3]);
    expect(index.nextBreak(0, onLine3)).toBe(2);
    expect(index.nextBreak(2, onLine3)).toBe(4);
    expect(index.nextBreak(4, onLine3)).toBe(8); // none left: go to the end
    expect(index.prevBreak(8, onLine3)).toBe(4);
    expect(index.prevBreak(2, onLine3)).toBe(0); // none before: go to the start
  });

  it("works on straight-line code, and ends at the final step", () => {
    const flat = StepIndex.fromTrace(straightLine.trace, straightLine.debug, lineOf);
    expect(flat.last).toBe(4);
    expect(flat.nextOver(1)).toBe(2);
    expect(flat.nextOut(1)).toBe(4);
    expect(flat.nextOver(3)).toBe(4);
    expect(flat.nextOver(4)).toBe(4);
  });
});
