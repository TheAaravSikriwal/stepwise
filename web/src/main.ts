// The playground page: editor, Run, and the debugger panels.

import { createEditor } from "./editor/editor";
import { EXAMPLES } from "./examples";
import { DEMO_DEBUG, DEMO_SOURCE, createDemoReplay, createDemoStepIndex } from "./replay/demo";
import { createReplay } from "./replay/replay";
import { describeOutcome } from "./runtime/run";
import { outputOf } from "./runtime/trace";
import { decodeProgram, encodeProgram } from "./share";
import { Debugger } from "./ui/debugger";
import { StepIndex } from "./ui/steps";
import { Runner } from "./worker/client";

const $ = <T extends HTMLElement>(sel: string) => document.querySelector<T>(sel)!;

const params = new URLSearchParams(location.search);
// `?demo` previews the debugger with a canned session (no compiler or replay needed).
const demo = params.has("demo");
// `?embed`: shown inside wearechintu.com/stepwise, which supplies the title and navigation.
if (params.has("embed")) document.documentElement.classList.add("embedded");
const DEFAULT_EXAMPLE = "factorial";

/** What to show on load: a shared program, `?example=<id>`, or the default example. */
async function initialSource(): Promise<string> {
  if (demo) return DEMO_SOURCE;
  const shared = await decodeProgram(location.hash);
  if (shared !== null) return shared;
  const id = params.get("example") ?? DEFAULT_EXAMPLE;
  return (EXAMPLES.find((e) => e.id === id) ?? EXAMPLES.find((e) => e.id === DEFAULT_EXAMPLE))?.source ?? "";
}

const output = $<HTMLPreElement>("#output");
const runButton = $<HTMLButtonElement>("#run");
const runner = new Runner();
const editor = createEditor($("#editor"), await initialSource(), runProgram);
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
    prevBreak: $<HTMLButtonElement>("#prev-break"),
    nextBreak: $<HTMLButtonElement>("#next-break"),
    over: $<HTMLButtonElement>("#step-over"),
    out: $<HTMLButtonElement>("#step-out"),
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
      const index = createDemoStepIndex(editor.lineOf);
      debug.load({ replay: createDemoReplay(), debug: DEMO_DEBUG, index, endMessage: null });
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
          const index = StepIndex.fromTrace(res.trace, res.meta.debug, editor.lineOf);
          debug.load({ replay, debug: res.meta.debug, index, endMessage });
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

// Example gallery
const examples = $<HTMLSelectElement>("#examples");
examples.append(new Option("Examples…", ""), ...EXAMPLES.map((e) => new Option(e.title, e.id)));
examples.addEventListener("change", () => {
  const example = EXAMPLES.find((e) => e.id === examples.value);
  examples.value = "";
  if (!example) return;
  editor.setSource(example.source); // Ctrl+Z brings back what was there
  history.replaceState(null, "", `?example=${example.id}`);
  output.replaceChildren();
  editor.showDiagnostics([]);
});

// Share: put the program in the URL and copy it
const shareStatus = $("#share-status");
$<HTMLButtonElement>("#share").addEventListener("click", async () => {
  const url = new URL(location.href);
  url.search = "";
  url.hash = await encodeProgram(editor.source());
  history.replaceState(null, "", url);
  try {
    await navigator.clipboard.writeText(url.href);
    shareStatus.textContent = "Link copied";
  } catch {
    shareStatus.textContent = "Copy the link from the address bar";
  }
  setTimeout(() => (shareStatus.textContent = ""), 3000);
});

// Dev-only handle for poking at the page from the browser console.
if (import.meta.env.DEV) Object.assign(window, { __stepwise: { editor, runner, debug, runProgram } });
