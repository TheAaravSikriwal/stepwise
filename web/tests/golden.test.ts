// Runs every golden program (compiler/tests/programs) through the *browser*
// runtime, not wasmtime, so the JS host functions are checked against the
// same expectations as the Rust tests. Skipped until the real compiler
// exists (PIPELINE = Codegen).

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import * as compilerModule from "../pkg/stepwise_wasm.js";
import { compileWith } from "../src/compiler";
import { type Outcome, run } from "../src/runtime/run";
import { outputOf } from "../src/runtime/trace";

const wasmPath = fileURLToPath(new URL("../pkg/stepwise_wasm_bg.wasm", import.meta.url));
compilerModule.initSync({ module: readFileSync(wasmPath) });
const realCompiler = compilerModule.pipeline() === "Codegen";

const dir = fileURLToPath(new URL("../../compiler/tests/programs/", import.meta.url));
const programs = readdirSync(dir)
  .filter((f) => f.endsWith(".step"))
  .sort()
  .map((name) => {
    const source = readFileSync(dir + name, "utf8");
    const lines = source.split(/\r?\n/);
    const output = lines.filter((l) => l.startsWith("// expect: ")).map((l) => l.slice(11));
    const trap = lines.find((l) => l.startsWith("// expect-trap: "))?.slice(16) ?? null;
    return { name, source, output, trap };
  });

/** The Rust tests name wasmtime's messages; the browser shows friendlier ones. */
function matchesTrap(trap: string, outcome: Outcome): boolean {
  if (trap === "stopped after") return outcome.kind === "limit";
  if (outcome.kind !== "error") return false;
  if (trap === "divide by zero") return /division by zero/i.test(outcome.message);
  if (trap === "stack") return /recursion/i.test(outcome.message);
  throw new Error(`golden.test.ts doesn't know how to check the trap ${JSON.stringify(trap)}`);
}

beforeAll(() => {
  expect(programs.length).toBeGreaterThanOrEqual(20);
});

describe.skipIf(!realCompiler)("golden programs in the browser runtime", () => {
  for (const p of programs) {
    it(p.name, async () => {
      const compiled = compileWith(compilerModule, p.source);
      expect(compiled.diagnostics.filter((d) => d.severity === "error")).toEqual([]);
      const { trace, outcome } = await run(compiled.wasm!);
      expect(outputOf(trace)).toEqual(p.output);
      if (p.trap) expect(matchesTrap(p.trap, outcome), JSON.stringify(outcome)).toBe(true);
      else expect(outcome).toEqual({ kind: "ok" });
      // Every `line` event must refer to a real step.
      for (let i = 0; i < trace.lines.length; i++) {
        expect(trace.a[trace.lines[i]]).toBeLessThan(compiled.debug.steps.length);
      }
    });
  }
});
