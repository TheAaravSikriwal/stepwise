// The debugger panels and controls: call stack, variables, output synced to
// the current step, step buttons, timeline, and keyboard shortcuts. Talks to
// the program only through the `Replay` interface.

import type { DebugTable } from "../compiler";
import type { Editor } from "../editor/editor";
import { type Replay, type ReplayState, varKey } from "../replay/types";

export interface Session {
  replay: Replay;
  debug: DebugTable;
  /** Shown at the final step, e.g. a runtime error or the step-limit message. */
  endMessage: string | null;
}

interface Elements {
  output: HTMLElement;
  stack: HTMLElement;
  vars: HTMLElement;
  stepLabel: HTMLElement;
  timeline: HTMLInputElement;
  toStart: HTMLButtonElement;
  back: HTMLButtonElement;
  forward: HTMLButtonElement;
  toEnd: HTMLButtonElement;
}

export class Debugger {
  private session: Session | null = null;
  private current: ReplayState | null = null;
  /** Index into `frames` of the frame whose variables are shown. */
  private selectedFrame = 0;

  constructor(
    private readonly el: Elements,
    private readonly editor: Editor,
  ) {
    el.toStart.addEventListener("click", () => this.seek(0));
    el.back.addEventListener("click", () => this.step(-1));
    el.forward.addEventListener("click", () => this.step(1));
    el.toEnd.addEventListener("click", () => this.seek(Infinity));
    el.timeline.addEventListener("input", () => this.seek(Number(el.timeline.value)));
    document.addEventListener("keydown", (e) => this.onKey(e));
    this.clear("Press Run, then step through your program with ← and →.");
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

  /** Shows a run's output without stepping (no replay available). */
  showOutputOnly(output: string[], endMessage: string | null, reason: string): void {
    this.stop();
    this.renderOutput(output, endMessage, true);
    this.el.stack.replaceChildren(notice(reason));
    this.el.vars.replaceChildren();
  }

  /** Leaves debug mode, showing `message` in the panels. */
  clear(message: string): void {
    this.stop();
    this.el.stack.replaceChildren(notice(message));
    this.el.vars.replaceChildren();
  }

  /** Leaves debug mode but keeps the output panel as it is. */
  private stop(): void {
    this.session = null;
    this.current = null;
    this.editor.showStep(null);
    this.setEnabled(false);
    this.el.stepLabel.textContent = "";
    this.el.timeline.value = "0";
  }

  step(delta: number): void {
    if (this.current) this.seek(this.current.step + delta);
  }

  seek(step: number): void {
    const s = this.session;
    if (!s) return;
    s.replay.seek(Math.max(0, Math.min(step, s.replay.stepCount - 1)));
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

    this.el.stepLabel.textContent = atEnd ? `Finished (${last} steps)` : `Step ${st.step + 1} of ${last}`;
    this.el.timeline.value = String(st.step);
    this.el.toStart.disabled = this.el.back.disabled = st.step === 0;
    this.el.forward.disabled = this.el.toEnd.disabled = atEnd;

    this.editor.showStep(st.line === null ? null : (s.debug.steps[st.line] ?? null));
    this.renderStack(st, s.debug);
    this.renderVars(st);
    this.renderOutput(st.output, s.endMessage, atEnd);
  }

  private renderStack(st: ReplayState, debug: DebugTable): void {
    if (st.frames.length === 0) {
      this.el.stack.replaceChildren(notice("The program has finished."));
      return;
    }
    const items = st.frames
      .map((f, i) => {
        const li = document.createElement("div");
        li.className = "frame";
        const line = f.line === null ? "" : ` line ${this.editor.lineOf(debug.steps[f.line]?.from ?? 0)}`;
        const name = document.createElement("span");
        name.className = "fn";
        name.textContent = f.name;
        const where = document.createElement("span");
        where.className = "where";
        where.textContent = line;
        li.append(name, where);
        li.tabIndex = 0;
        li.setAttribute("role", "button");
        li.setAttribute("aria-pressed", String(i === this.selectedFrame));
        if (i === this.selectedFrame) li.classList.add("selected");
        const select = () => {
          this.selectedFrame = i;
          this.renderStack(st, debug);
          this.renderVars(st);
        };
        li.addEventListener("click", select);
        li.addEventListener("keydown", (e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            select();
          }
        });
        return li;
      })
      .reverse(); // innermost on top
    this.el.stack.replaceChildren(...items);
  }

  private renderVars(st: ReplayState): void {
    const frame = st.frames[this.selectedFrame];
    if (!frame) {
      this.el.vars.replaceChildren();
      return;
    }
    if (frame.vars.length === 0) {
      this.el.vars.replaceChildren(notice(`No variables in ${frame.name} yet.`));
      return;
    }
    const table = document.createElement("table");
    for (const v of frame.vars) {
      const row = table.insertRow();
      if (st.changed.has(varKey(this.selectedFrame, v.varId))) {
        row.className = "changed";
        row.title = "Changed by the last step";
      }
      row.insertCell().textContent = v.name;
      row.insertCell().textContent = v.display;
    }
    this.el.vars.replaceChildren(table);
  }

  private renderOutput(lines: string[], endMessage: string | null, atEnd: boolean): void {
    const nodes = lines.map((text) => div(text));
    if (atEnd && endMessage) nodes.push(div(endMessage, "error"));
    this.el.output.replaceChildren(...nodes);
    this.el.output.scrollTop = this.el.output.scrollHeight;
  }

  private setEnabled(on: boolean): void {
    for (const b of [this.el.toStart, this.el.back, this.el.forward, this.el.toEnd]) b.disabled = !on;
    this.el.timeline.disabled = !on;
  }

  private onKey(e: KeyboardEvent): void {
    if (!this.active || e.ctrlKey || e.metaKey) return;
    const inEditor = (e.target as HTMLElement | null)?.closest?.(".cm-editor") != null;
    // In the editor, plain arrows move the cursor; Alt+arrows always step.
    if (inEditor && !e.altKey) return;
    const actions: Record<string, () => void> = {
      ArrowLeft: () => this.step(-1),
      ArrowRight: () => this.step(1),
      Home: () => this.seek(0),
      End: () => this.seek(Infinity),
    };
    const action = actions[e.key];
    if (!action || (e.target instanceof HTMLInputElement && e.target.type === "range" && !e.altKey)) return;
    e.preventDefault();
    action();
  }
}

function div(text: string, className?: string): HTMLElement {
  const el = document.createElement("div");
  el.textContent = text;
  if (className) el.className = className;
  return el;
}

function notice(text: string): HTMLElement {
  return div(text, "notice");
}
