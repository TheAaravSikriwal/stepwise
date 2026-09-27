// Phase 0 page: a plain textarea and an output panel. The CodeMirror editor
// and debugger panels replace this in later phases.

import { describeOutcome } from "./runtime/run";
import { outputOf } from "./runtime/trace";
import { Runner } from "./worker/client";

const EXAMPLE = `fn main() {
    print(42);
}
`;

const source = document.querySelector<HTMLTextAreaElement>("#source")!;
const output = document.querySelector<HTMLPreElement>("#output")!;
const runButton = document.querySelector<HTMLButtonElement>("#run")!;
const runner = new Runner();

source.value = EXAMPLE;

function line(text: string, className?: string): HTMLElement {
  const el = document.createElement("div");
  el.textContent = text;
  if (className) el.className = className;
  return el;
}

async function runProgram(): Promise<void> {
  runButton.disabled = true;
  try {
    const res = await runner.run(source.value);
    if (res.type === "internal-error" && res.message === "cancelled") return;
    output.replaceChildren();
    switch (res.type) {
      case "compile-error":
        for (const d of res.meta.diagnostics) output.append(line(d.rendered, "error"));
        break;
      case "ran": {
        for (const text of outputOf(res.trace)) output.append(line(text));
        const problem = describeOutcome(res.outcome);
        if (problem) output.append(line(problem, "error"));
        const steps = res.trace.lines.length;
        output.append(line(`${steps} ${steps === 1 ? "step" : "steps"} recorded`, "notice"));
        break;
      }
      case "internal-error":
        output.append(line(res.message, "error"));
        break;
    }
  } finally {
    runButton.disabled = false;
  }
}

runButton.addEventListener("click", runProgram);
source.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
    e.preventDefault();
    runProgram();
  }
});
