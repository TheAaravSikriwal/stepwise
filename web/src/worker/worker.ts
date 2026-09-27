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
    scope.postMessage({ type: "ran", id: req.id, meta, trace, outcome }, transferables(trace));
  } catch (e) {
    scope.postMessage({ type: "internal-error", id: req.id, message: String(e) });
  }
};
