// The debugger: the step buttons, the "what just happened" explainer, and
// the Variables, Printed output and Functions running panels. It is written
// for someone who has never used a debugger: every panel says what it is
// for, every empty panel says what will appear there, and the explainer says
// in plain English what the last step did and what happens next.
//
// Talks to the program only through the `Replay` interface.

import type { DebugTable, Range } from "../compiler";
import type { Editor } from "../editor/editor";
import { type Replay, type ReplayState, varKey } from "../replay/types";
import type { TraceData } from "../runtime/trace";
import { narrate } from "./narrate";
import type { StepIndex } from "./steps";

export interface Session {
  replay: Replay;
  debug: DebugTable;
  /** The program that ran, for naming things in the explainer. */
  source: string;
  /** For skipping over calls, finishing functions, and stops. */
  index: StepIndex;
  /** Shown at the final step, e.g. a runtime error or the step-limit message. */
  endMessage: string | null;
  /** The recording, for the call galaxy. The demo has none. */
  trace?: TraceData;
}

interface Elements {
  explain: HTMLElement;
  output: HTMLElement;
  stack: HTMLElement;
  vars: HTMLElement;
  stepLabel: HTMLElement;
  timeline: HTMLInputElement;
  toStart: HTMLButtonElement;
  back: HTMLButtonElement;
  forward: HTMLButtonElement;
  toEnd: HTMLButtonElement;
  prevBreak: HTMLButtonElement;
  nextBreak: HTMLButtonElement;
  over: HTMLButtonElement;
  out: HTMLButtonElement;
}

type Tone = "start" | "step" | "done" | "problem";

interface Explanation {
  tone: Tone;
  label: string;
  lines: string[];
  next?: string | null;
  tip?: string;
  /** A button offering the obvious fix. */
  action?: { label: string; run: () => void };
}

const TIPS = [
  "Tip: press → on your keyboard for the next step, and ← to go back.",
  "Tip: the gold line is the one about to run. Nothing on it has happened yet.",
  "Tip: variables highlighted in gold were just made or changed.",
  "Tip: drag the slider to move through the whole run at once.",
  "Tip: click just left of a line number to put a stop there. Then use More moves to jump to it.",
];

export class Debugger {
  private session: Session | null = null;
  private current: ReplayState | null = null;
  private previous: ReplayState | null = null;
  /** Index into `frames` of the frame whose variables are shown. */
  private selectedFrame = 0;
  /** Set by "where from?" until the next move: which value, and the line that set it. */
  private origin: { name: string; value: string; span: Range | null; passedBy: string | null } | null = null;
  private listeners: ((session: Session | null, state: ReplayState | null) => void)[] = [];

  constructor(
    private readonly el: Elements,
    private readonly editor: Editor,
  ) {
    el.toStart.addEventListener("click", () => this.seek(0));
    el.back.addEventListener("click", () => this.step(-1));
    el.forward.addEventListener("click", () => this.step(1));
    el.toEnd.addEventListener("click", () => this.seek(Infinity));
    el.prevBreak.addEventListener("click", () => this.toBreakpoint(-1));
    el.nextBreak.addEventListener("click", () => this.toBreakpoint(1));
    // Shift+click goes backward.
    el.over.addEventListener("click", (e) => this.stepOver(e.shiftKey ? -1 : 1));
    el.out.addEventListener("click", (e) => this.stepOut(e.shiftKey ? -1 : 1));
    el.timeline.addEventListener("input", () => this.seek(Number(el.timeline.value)));
    document.addEventListener("keydown", (e) => this.onKey(e));
    this.idle();
  }

  /** Called after every move with the new state, and with nulls when debugging stops. */
  onStep(listener: (session: Session | null, state: ReplayState | null) => void): void {
    this.listeners.push(listener);
  }

  get active(): boolean {
    return this.session !== null;
  }

  /** Starts debugging a finished run, at step 0. */
  load(session: Session): void {
    this.session = session;
    this.el.timeline.max = String(session.replay.stepCount - 1);
    this.setEnabled(true);
    this.seek(0);
  }

  /** Before anything has run. */
  idle(): void {
    this.stop();
    this.emptyPanels();
    this.el.output.replaceChildren(notice("Anything your program prints will show up here."));
    this.explain({
      tone: "start",
      label: "How to start",
      lines: [
        "Press ▶ Run to start your program.",
        "Then press Next step to go through it one line at a time, and watch what each line does.",
      ],
      tip: "Tip: open the examples menu above your code to try a ready-made program.",
    });
  }

  /** The code was edited after a run, so the recording no longer matches it. */
  codeChanged(): void {
    this.stop();
    this.emptyPanels();
    this.explain({
      tone: "start",
      label: "Your code changed",
      lines: ["Press ▶ Run to try the new version."],
    });
  }

  /** The compiler found problems, listed in the output panel by the caller. */
  problems(count: number): void {
    this.stop();
    this.emptyPanels();
    const what = count === 1 ? "a problem" : `${count} problems`;
    this.explain({
      tone: "problem",
      label: count === 1 ? "Your code has a problem" : "Your code has some problems",
      lines: [
        `Stepwise found ${what} before running your program.`,
        "The red underline in your code shows where. The Output panel says what's wrong and how to fix it.",
        "Fix it, then press ▶ Run again.",
      ],
    });
  }

  /** Demo mode (`?demo`), but the editor holds something other than the demo's program. */
  demoOnly(): void {
    this.stop();
    this.emptyPanels();
    this.el.output.replaceChildren(notice("Nothing ran."));
    this.explain({
      tone: "problem",
      label: "This is the demo page",
      lines: [
        "The demo only plays its own small example, not the code in the editor.",
        "To run and step through your own code, use the normal playground.",
      ],
      action: {
        label: "Open the playground",
        run: () => {
          // Keep ?embed and ?theme, so it still fits a page it's framed in.
          const url = new URL(location.href);
          url.searchParams.delete("demo");
          location.assign(url);
        },
      },
    });
  }

  /** Something went wrong inside Stepwise itself (the message is in the output panel). */
  failed(): void {
    this.stop();
    this.emptyPanels();
    this.explain({
      tone: "problem",
      label: "Something went wrong",
      lines: [
        "This isn't a problem with your code: Stepwise itself hit a bug. The details are in the Output panel.",
        "Press ▶ Run to try again.",
      ],
    });
  }

  /** A run's output without stepping (no replay available). */
  showOutputOnly(output: string[], endMessage: string | null, reason: string): void {
    this.stop();
    this.emptyPanels();
    this.renderOutput(output, endMessage, true);
    this.explain({
      tone: "done",
      label: "Your program ran",
      lines: ["What it printed is in the Output panel.", `Stepping through it isn't available yet: ${reason}`],
    });
  }

  /** Leaves debug mode but keeps the output panel as it is. */
  private stop(): void {
    this.session = null;
    this.current = null;
    this.previous = null;
    this.editor.showStep(null);
    this.setEnabled(false);
    this.el.stepLabel.textContent = "";
    this.el.timeline.value = "0";
    this.el.timeline.style.setProperty("--p", "0%");
    for (const l of this.listeners) l(null, null);
  }

  step(delta: number): void {
    if (this.current) this.seek(this.current.step + delta);
  }

  /** Steps without entering function calls. */
  stepOver(dir: 1 | -1): void {
    this.jump((ix, k) => (dir === 1 ? ix.nextOver(k) : ix.prevOver(k)));
  }

  /** Finishes the current function (or, backward, returns to where it was called). */
  stepOut(dir: 1 | -1): void {
    this.jump((ix, k) => (dir === 1 ? ix.nextOut(k) : ix.prevOut(k)));
  }

  /** Runs to the next (or previous) step on a line with a stop. */
  toBreakpoint(dir: 1 | -1): void {
    const lines = this.editor.breakpoints();
    if (lines.size === 0) {
      this.flashTip("There are no stops yet. Click just left of a line number to put one there.");
      return;
    }
    this.jump((ix, k) => (dir === 1 ? ix.nextBreak(k, lines) : ix.prevBreak(k, lines)));
  }

  private jump(target: (index: StepIndex, step: number) => number): void {
    if (this.session && this.current) this.seek(target(this.session.index, this.current.step));
  }

  /**
   * "Where from?": jumps to the moment a variable got its current value,
   * outlines the line that set it, and says so in the explainer.
   */
  whereFrom(frameIndex: number, varId: number): void {
    const s = this.session;
    const frame = this.current?.frames[frameIndex];
    const v = frame?.vars.find((x) => x.varId === varId);
    if (!s || !frame || !v) return;
    const found = s.replay.origin(frame.callId, varId);
    if (!found) {
      this.flashTip(`Couldn't find where \`${v.name}\` got its value.`);
      return;
    }
    s.replay.seek(found.statement);
    const at = s.replay.state();
    // If the variable's call didn't exist yet at that statement, the
    // statement was the call that passed it in: a parameter.
    const passedBy = at.frames.length <= frameIndex ? frame.name : null;
    const span = at.line === null ? null : (s.debug.steps[at.line] ?? null);
    this.origin = { name: v.name, value: v.display, span, passedBy };
    this.seek(found.after, true);
  }

  seek(step: number, keepOrigin = false): void {
    const s = this.session;
    if (!s) return;
    if (!keepOrigin) this.origin = null;
    const k = Math.max(0, Math.min(Math.trunc(step) || 0, s.replay.stepCount - 1));
    // The step before, so the explainer can say what changed.
    this.previous = null;
    if (k > 0) {
      s.replay.seek(k - 1);
      this.previous = s.replay.state();
    }
    s.replay.seek(k);
    this.current = s.replay.state();
    this.selectedFrame = this.current.frames.length - 1;
    this.render();
  }

  private render(): void {
    const s = this.session;
    const st = this.current;
    if (!s || !st) return;
    const last = s.replay.stepCount - 1;
    const atEnd = st.step === last;

    const count = (n: number) => n.toLocaleString("en-US");
    this.el.stepLabel.textContent = atEnd
      ? `${s.endMessage ? "Stopped" : "Finished"} · ${count(last)} steps`
      : `Step ${count(st.step + 1)} of ${count(last)}`;
    this.el.timeline.value = String(st.step);
    this.el.timeline.style.setProperty("--p", `${last > 0 ? (st.step / last) * 100 : 0}%`);
    this.el.toStart.disabled = this.el.back.disabled = this.el.prevBreak.disabled = st.step === 0;
    this.el.forward.disabled = this.el.toEnd.disabled = this.el.nextBreak.disabled = atEnd;

    this.editor.showStep(st.line === null ? null : (s.debug.steps[st.line] ?? null));
    this.editor.showOrigin(this.origin?.span ?? null);
    this.renderStack(st, s.debug);
    this.renderVars(st);
    this.renderOutput(st.output, s.endMessage, atEnd);
    for (const l of this.listeners) l(s, st);

    const n = narrate(this.previous, st, { debug: s.debug, source: s.source, lineOf: this.editor.lineOf }, s.endMessage);
    if (this.origin) {
      const { name, value, span, passedBy } = this.origin;
      const where = span ? `Line ${this.editor.lineOf(span.from)}` : "This step";
      const how = passedBy
        ? `called \`${passedBy}\` and passed in \`${name}\` = ${value}`
        : `gave \`${name}\` its value, ${value}`;
      this.explain({
        tone: "step",
        label: `Where \`${name}\` came from`,
        lines: [`${where} ${how}. It's the outlined line in your code.`, ...n.happened],
        next: n.next,
        tip: "Press Next step or Back to carry on from here.",
      });
      return;
    }
    this.explain({
      tone: atEnd ? (s.endMessage ? "problem" : "done") : st.step === 0 ? "start" : "step",
      label: atEnd ? (s.endMessage ? "The program stopped" : "Finished") : st.step === 0 ? "Ready" : "What just happened",
      lines: n.happened,
      next: n.next,
      tip: atEnd
        ? "Press ◀ Back or drag the slider to look at any moment of the run again."
        : TIPS[Math.floor(st.step / 4) % TIPS.length],
    });
  }

  private explain(e: Explanation): void {
    const box = this.el.explain;
    box.dataset.tone = e.tone;
    const label = rich("div", e.label, "explain-label");
    const list = document.createElement("ul");
    list.className = "explain-lines";
    for (const line of e.lines) list.append(rich("li", line));
    const nodes: Node[] = [label, list];
    if (e.next) nodes.push(rich("p", e.next, "explain-next"));
    if (e.action) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "explain-action";
      button.textContent = e.action.label;
      button.addEventListener("click", e.action.run);
      nodes.push(button);
    }
    if (e.tip) nodes.push(rich("p", e.tip, "explain-tip"));
    box.replaceChildren(...nodes);
  }

  /** Shows a one-off tip in the explainer without losing the rest of it. */
  private flashTip(text: string): void {
    const tip = this.el.explain.querySelector(".explain-tip") ?? this.el.explain.appendChild(el("p", "explain-tip", ""));
    tip.replaceWith(rich("p", text, "explain-tip attention"));
  }

  private emptyPanels(): void {
    this.el.vars.replaceChildren(notice("Your variables will show up here while the program runs."));
    this.el.stack.replaceChildren(
      notice("When your program runs, this shows which function is running, and which one called it."),
    );
  }

  private renderStack(st: ReplayState, debug: DebugTable): void {
    if (st.frames.length === 0) {
      this.el.stack.replaceChildren(notice("Nothing is running: the program has finished."));
      return;
    }
    const innermost = st.frames.length - 1;
    const items = st.frames
      .map((f, i) => {
        const item = document.createElement("div");
        item.className = "frame";
        const name = el("span", "fn", f.name);
        const where = el(
          "span",
          "where",
          f.line === null ? "" : `line ${this.editor.lineOf(debug.steps[f.line]?.from ?? 0)}`,
        );
        const role = el("span", "role", i === innermost ? "running now" : `waiting for ${st.frames[i + 1].name}`);
        item.append(name, where, role);
        item.tabIndex = 0;
        item.setAttribute("role", "button");
        item.title = "Show this function's variables";
        item.setAttribute("aria-pressed", String(i === this.selectedFrame));
        if (i === this.selectedFrame) item.classList.add("selected");
        if (i === innermost) item.classList.add("current");
        const select = () => {
          this.selectedFrame = i;
          this.renderStack(st, debug);
          this.renderVars(st);
        };
        item.addEventListener("click", select);
        item.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            select();
          }
        });
        return item;
      })
      .reverse(); // newest on top
    this.el.stack.replaceChildren(...items);
  }

  private renderVars(st: ReplayState): void {
    const frame = st.frames[this.selectedFrame];
    if (!frame) {
      this.el.vars.replaceChildren(
        notice("The program has finished, so its variables are gone. Press ◀ Back to see them again."),
      );
      return;
    }
    // Which call's variables these are, and at which moment.
    const caption = rich("p", `\`${frame.name}\`'s variables at `, "vars-caption");
    caption.append(el("span", "at", `step ${(st.step + 1).toLocaleString("en-US")}`), ".");
    if (st.frames.length > 1) caption.append(" Click a function under Functions running to see another one's.");
    const nodes: Node[] = [caption];
    if (frame.vars.length === 0) {
      nodes.push(rich("div", `\`${frame.name}\` hasn't made any variables yet.`, "notice"));
      this.el.vars.replaceChildren(...nodes);
      return;
    }
    const table = document.createElement("table");
    const head = table.createTHead().insertRow();
    for (const label of ["Name", "Value", ""]) {
      const th = document.createElement("th");
      th.textContent = label;
      head.append(th);
    }
    const body = table.createTBody();
    for (const v of frame.vars) {
      const row = body.insertRow();
      if (st.changed.has(varKey(this.selectedFrame, v.varId))) {
        row.className = "changed";
        row.title = "Just made or changed by the last step";
      }
      row.insertCell().textContent = v.name;
      row.insertCell().textContent = v.display;
      const button = document.createElement("button");
      button.type = "button";
      button.className = "where-from";
      button.textContent = "where from?";
      button.title = `Jump to the line that gave ${v.name} this value`;
      const frameIndex = this.selectedFrame;
      button.addEventListener("click", () => this.whereFrom(frameIndex, v.varId));
      row.insertCell().append(button);
    }
    nodes.push(table);
    this.el.vars.replaceChildren(...nodes);
  }

  private renderOutput(lines: string[], endMessage: string | null, atEnd: boolean): void {
    const nodes = lines.map((text) => el("div", "", text));
    if (atEnd && endMessage) nodes.push(el("div", "error", endMessage));
    if (nodes.length === 0) nodes.push(notice(atEnd ? "The program didn't print anything." : "Nothing printed yet."));
    this.el.output.replaceChildren(...nodes);
    this.el.output.scrollTop = this.el.output.scrollHeight;
  }

  private setEnabled(on: boolean): void {
    const buttons = [
      this.el.toStart,
      this.el.back,
      this.el.forward,
      this.el.toEnd,
      this.el.prevBreak,
      this.el.nextBreak,
      this.el.over,
      this.el.out,
    ];
    for (const b of buttons) b.disabled = !on;
    this.el.timeline.disabled = !on;
  }

  private onKey(e: KeyboardEvent): void {
    if (!this.active || e.ctrlKey || e.metaKey) return;
    const inEditor = (e.target as HTMLElement | null)?.closest?.(".cm-editor") != null;
    // In the editor, plain arrows move the cursor; Alt+arrows always step.
    if (inEditor && !e.altKey) return;
    const actions: Record<string, () => void> = e.shiftKey
      ? { ArrowLeft: () => this.stepOver(-1), ArrowRight: () => this.stepOver(1) }
      : {
          ArrowLeft: () => this.step(-1),
          ArrowRight: () => this.step(1),
          Home: () => this.seek(0),
          End: () => this.seek(Infinity),
          PageUp: () => this.toBreakpoint(-1),
          PageDown: () => this.toBreakpoint(1),
        };
    const action = actions[e.key];
    if (!action || (e.target instanceof HTMLInputElement && e.target.type === "range" && !e.altKey)) return;
    e.preventDefault();
    action();
  }
}

function el(tag: string, className: string, text: string): HTMLElement {
  const node = document.createElement(tag);
  if (className) node.className = className;
  node.textContent = text;
  return node;
}

/** Text with `backticks` turned into <code>, safely (never as HTML). */
function rich(tag: string, text: string, className = ""): HTMLElement {
  const node = document.createElement(tag);
  if (className) node.className = className;
  text.split(/(`[^`]*`)/).forEach((part) => {
    if (part.startsWith("`") && part.endsWith("`") && part.length >= 2) {
      const c = document.createElement("code");
      c.textContent = part.slice(1, -1);
      node.append(c);
    } else if (part) {
      node.append(part);
    }
  });
  return node;
}

function notice(text: string): HTMLElement {
  return el("div", "notice", text);
}
