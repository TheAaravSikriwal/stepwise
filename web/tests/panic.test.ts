// A compiler panic must reach JS with its real message, not just
// "unreachable". Separate file because a panic leaves the compiler instance
// unusable, and each test file gets a fresh one.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";
import * as compilerModule from "../pkg/stepwise_wasm.js";

it("reports the panic message through the hook", () => {
  let reported: string | null = null;
  Object.assign(globalThis, { __stepwisePanic: (m: string) => (reported = m) });
  const wasmPath = fileURLToPath(new URL("../pkg/stepwise_wasm_bg.wasm", import.meta.url));
  compilerModule.initSync({ module: readFileSync(wasmPath) });

  expect(() => compilerModule.test_panic()).toThrow();
  expect(reported).toMatch(/test panic requested/);
  expect(reported).toMatch(/lib\.rs:\d+/); // includes where it happened
});
