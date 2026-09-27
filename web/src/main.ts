// The playground page: editor, Run, and the debugger panels.

import { createEditor } from "./editor/editor";
import { DEMO_DEBUG, DEMO_SOURCE, createDemoReplay } from "./replay/demo";
import { createReplay } from "./replay/replay";
import { describeOutcome } from "./runtime/run";
import { outputOf } from "./runtime/trace";
import { Debugger } from "./ui/debugger";
import { Runner } from "./worker/client";

const EXAMPLE = `fn factorial(n: int) -> int {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}

fn main() {
    let mut total = 0;
    let mut i = 1;
    while i <= 5 {
        total = total + factorial(i);
        i = i + 1;
    }
    print(total);
}
`;

const $ = <T extends HTMLElement>(sel: string) => document.querySelector<T>(sel)!;

// `?demo` previews the debugger with a canned session (no compiler or replay needed).
const demo = new URLSearchParams(location.search).has("demo");

const output = $<HTMLPreElement>("#output");
const runButton = $<HTMLButtonElement>("#run");
const runner = new Runner();
const editor = createEditor($("#editor"), demo ? DEMO_SOURCE : EXAMPLE, runProgram);
const debug = new Debugger(
  {
    output,
    stack: $("#stack"),
    vars: $("#vars"),
    stepLabel: $("#step-label"),
    timeline: $<HTMLInputElement>("#timeline"),
    toStart: $<HTMLButtonElement>("#to-start"),
    back: $<HTMLButtonElement>("#step-back"),
    forward: $<HTMLButtonElement>("#step-forward"),
    toEnd: $<HTMLButtonElement>("#to-end"),
  },
  editor,
);

// A recording only matches the code it came from.
editor.onChange(() => {
  if (debug.active) debug.clear("The code changed. Press Run to debug it again.");
});

function line(text: string, className?: string): HTMLElement {
  const el = document.createElement("div");
  el.textContent = text;
  if (className) el.className = className;
  return el;
}

async function runProgram(): Promise<void> {
  runButton.disabled = true;
  try {
    if (demo) {
      debug.load({ replay: createDemoReplay(), debug: DEMO_DEBUG, endMessage: null });
      return;
    }
    const res = await runner.run(editor.source());
    if (res.type === "internal-error" && res.message === "cancelled") return;
    switch (res.type) {
      case "compile-error":
        editor.showDiagnostics(res.meta.diagnostics);
        debug.clear("Fix the errors in your code, then press Run again.");
        output.replaceChildren(...res.meta.diagnostics.map((d) => line(d.rendered, "error")));
        break;
      case "ran": {
        editor.showDiagnostics(res.meta.diagnostics); // warnings, if any
        const endMessage = describeOutcome(res.outcome);
        try {
          const replay = createReplay(res.trace, res.meta.debug);
          debug.load({ replay, debug: res.meta.debug, endMessage });
        } catch (e) {
          // No replay engine yet (or it crashed): still show the output.
          debug.showOutputOnly(outputOf(res.trace), endMessage, `Stepping isn't available: ${(e as Error).message}`);
        }
        break;
      }
      case "internal-error":
        editor.showDiagnostics([]);
        debug.clear("Something went wrong. See the output panel.");
        output.replaceChildren(line(res.message, "error"));
        break;
    }
  } finally {
    runButton.disabled = false;
  }
}

runButton.addEventListener("click", runProgram);

// Dev-only handle for poking at the page from the browser console.
if (import.meta.env.DEV) Object.assign(window, { __stepwise: { editor, runner, debug, runProgram } });
