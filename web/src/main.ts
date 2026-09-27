// The playground page: editor, Run button, output panel. The debugger panels
// arrive in Phase 5.

import { createEditor } from "./editor/editor";
import { describeOutcome } from "./runtime/run";
import { outputOf } from "./runtime/trace";
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

const output = document.querySelector<HTMLPreElement>("#output")!;
const runButton = document.querySelector<HTMLButtonElement>("#run")!;
const runner = new Runner();
const editor = createEditor(document.querySelector("#editor")!, EXAMPLE, runProgram);

function line(text: string, className?: string): HTMLElement {
  const el = document.createElement("div");
  el.textContent = text;
  if (className) el.className = className;
  return el;
}

async function runProgram(): Promise<void> {
  runButton.disabled = true;
  try {
    const res = await runner.run(editor.source());
    if (res.type === "internal-error" && res.message === "cancelled") return;
    output.replaceChildren();
    switch (res.type) {
      case "compile-error":
        editor.showDiagnostics(res.meta.diagnostics);
        for (const d of res.meta.diagnostics) output.append(line(d.rendered, "error"));
        break;
      case "ran": {
        editor.showDiagnostics(res.meta.diagnostics); // warnings, if any
        for (const text of outputOf(res.trace)) output.append(line(text));
        const problem = describeOutcome(res.outcome);
        if (problem) output.append(line(problem, "error"));
        const steps = res.trace.lines.length;
        output.append(line(`${steps} ${steps === 1 ? "step" : "steps"} recorded`, "notice"));
        break;
      }
      case "internal-error":
        editor.showDiagnostics([]);
        output.append(line(res.message, "error"));
        break;
    }
  } finally {
    runButton.disabled = false;
  }
}

runButton.addEventListener("click", runProgram);

// Dev-only handle for poking at the page from the browser console.
if (import.meta.env.DEV) Object.assign(window, { __stepwise: { editor, runner, runProgram } });
