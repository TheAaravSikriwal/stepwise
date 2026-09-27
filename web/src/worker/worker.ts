// Compiles and runs programs off the main thread, so a slow program can
// never freeze the page.

import init, * as compilerModule from "../../pkg/stepwise_wasm.js";
import { compileWith } from "../compiler";
import { run } from "../runtime/run";
import { transferables } from "../runtime/trace";
import type { Request, Response } from "./protocol";

// The DOM typings describe `window`, not a worker, so describe what we use.
const scope = self as unknown as {
  onmessage: ((e: MessageEvent<Request>) => void) | null;
  postMessage(message: Response, transfer?: Transferable[]): void;
  __stepwisePanic?: (message: string) => void;
};

// The compiler's panic hook calls this (see compiler-wasm/src/lib.rs).
let panicMessage: string | null = null;
scope.__stepwisePanic = (message) => {
  panicMessage = message;
};

const ready = init();

scope.onmessage = async ({ data: req }) => {
  try {
    await ready;
    const { wasm, ...meta } = compileWith(compilerModule, req.source);
    if (!wasm) {
      scope.postMessage({ type: "compile-error", id: req.id, meta });
      return;
    }
    const { trace, outcome } = await run(wasm);
    const realCompiler = compilerModule.pipeline() === "Codegen";
    scope.postMessage({ type: "ran", id: req.id, meta, trace, outcome, realCompiler }, transferables(trace));
  } catch (e) {
    const message = panicMessage
      ? `The compiler crashed. This is a bug in Stepwise, not in your program.\n\n${panicMessage}`
      : `Something went wrong inside Stepwise: ${e}`;
    // After a panic the compiler's memory may be inconsistent, so the page
    // replaces this worker when it gets an internal error.
    scope.postMessage({ type: "internal-error", id: req.id, message });
  }
};
