// Replay performance: jumping anywhere in a long run must feel instant.
// Budgets are generous (CI machines are slow), but replaying from the start
// on every seek won't meet them: you need snapshots (docs/specs/replay.md).

import { describe, expect, it } from "vitest";
import { createReplay } from "../src/replay/replay";
import { longLoop } from "./fixtures/traces";

// 250,000 iterations × 4 events = about a million events: the event limit.
const big = longLoop(250_000);

function time(fn: () => void): number {
  const start = performance.now();
  fn();
  return performance.now() - start;
}

/** Runs `step` up to `n` times, stopping early once over budget. Returns how many ran. */
function withinBudget(budgetMs: number, n: number, step: (k: number) => void): number {
  const start = performance.now();
  for (let k = 0; k < n; k++) {
    if (performance.now() - start > budgetMs) return k;
    step(k);
  }
  return performance.now() - start > budgetMs ? n - 1 : n;
}

describe("performance on a million-event trace", () => {
  it("builds in under 2 seconds", () => {
    expect(time(() => createReplay(big.trace, big.debug))).toBeLessThan(2000);
  });

  it("300 random jumps take under 1.5 seconds in total", () => {
    const r = createReplay(big.trace, big.debug);
    let seed = 42;
    const done = withinBudget(1500, 300, () => {
      seed = (seed * 1103515245 + 12345) % 2 ** 31;
      r.seek(seed % r.stepCount);
      r.state();
    });
    expect(done, "jumps completed within 1.5 s").toBe(300);
  });

  it("stepping backward one step at a time is fast", () => {
    const r = createReplay(big.trace, big.debug);
    r.seek(r.stepCount - 1);
    const done = withinBudget(1000, 5000, () => {
      r.seek(r.state().step - 1);
      r.state();
    });
    expect(done, "backward steps completed within 1 s").toBe(5000);
  });

  it("gives the right answer far into the run", () => {
    const r = createReplay(big.trace, big.debug);
    // Step 1 + 2k is the loop check after k passes, so i = k.
    r.seek(1 + 2 * 123_456);
    expect(r.state().frames[0].vars[0].display).toBe("123456");
    r.seek(r.stepCount - 1);
    expect(r.state().output).toEqual(["250000"]);
  });
});
