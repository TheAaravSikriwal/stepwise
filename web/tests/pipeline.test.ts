// End-to-end without a browser: real compiler (built by `npm run wasm`),
// real runtime, real trace.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { beforeAll, describe, expect, it } from "vitest";
import * as compilerModule from "../pkg/stepwise_wasm.js";
import { compileWith } from "../src/compiler";
import { run } from "../src/runtime/run";
import { Kind, outputOf } from "../src/runtime/trace";

beforeAll(() => {
  const wasmPath = fileURLToPath(new URL("../pkg/stepwise_wasm_bg.wasm", import.meta.url));
  compilerModule.initSync({ module: readFileSync(wasmPath) });
});

describe("phase 0 pipeline", () => {
  it("compiles, runs, and records the stub program", async () => {
    const compiled = compileWith(compilerModule, "fn main() { print(42); }");
    expect(compiled.diagnostics).toEqual([]);
    expect(compiled.debug.functions.map((f) => f.name)).toEqual(["main"]);

    const { trace, outcome } = await run(compiled.wasm!);
    expect(outcome).toEqual({ kind: "ok" });
    expect(outputOf(trace)).toEqual(["42"]);
    expect(Array.from(trace.kind)).toEqual([Kind.Call, Kind.Line, Kind.Print, Kind.ScopeExit, Kind.Ret]);
    expect(Array.from(trace.lines)).toEqual([1]);
  });

  it("stops cleanly at the event limit", async () => {
    const compiled = compileWith(compilerModule, "");
    const { trace, outcome } = await run(compiled.wasm!, 3);
    expect(outcome).toEqual({ kind: "limit", limit: 3 });
    expect(trace.length).toBe(3);
  });
});
