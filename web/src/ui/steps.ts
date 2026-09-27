// Step navigation that works purely from the recording: step over, step out,
// and breakpoints, in both directions. For each step we know its call depth
// and its source line, and every command is a scan for the next (or
// previous) step that matches.

import type { DebugTable } from "../compiler";
import { Kind, type TraceData } from "../runtime/trace";

export class StepIndex {
  /** The final "finished" step. */
  readonly last: number;

  /**
   * @param depth call depth at each step (1 = in `main`); the final step should be 0
   * @param line 1-based source line of each step; 0 for the final step
   */
  constructor(
    private readonly depth: Int32Array,
    private readonly line: Int32Array,
  ) {
    this.last = depth.length - 1;
  }

  static fromTrace(trace: TraceData, debug: DebugTable, lineOf: (offset: number) => number): StepIndex {
    const n = trace.lines.length;
    const depth = new Int32Array(n + 1);
    const line = new Int32Array(n + 1);
    let d = 0;
    let k = 0;
    for (let i = 0; i < trace.length; i++) {
      const kind = trace.kind[i];
      if (kind === Kind.Call) d++;
      else if (kind === Kind.Ret) d--;
      else if (kind === Kind.Line) {
        depth[k] = d;
        const span = debug.steps[trace.a[i]];
        line[k] = span ? lineOf(span.from) : 0;
        k++;
      }
    }
    return new StepIndex(depth, line);
  }

  /** Line number of a step (0 for the final step). */
  lineAt(step: number): number {
    return this.line[step] ?? 0;
  }

  /** The next step at the same depth or shallower: skips over calls. */
  nextOver(step: number): number {
    return this.find(step, 1, (j) => this.depth[j] <= this.depth[step]);
  }

  prevOver(step: number): number {
    return this.find(step, -1, (j) => this.depth[j] <= this.depth[step]);
  }

  /** The next step in a calling frame: finishes the current function. */
  nextOut(step: number): number {
    return this.find(step, 1, (j) => this.depth[j] < this.depth[step]);
  }

  /** The previous step in a calling frame: back to the call site. */
  prevOut(step: number): number {
    return this.find(step, -1, (j) => this.depth[j] < this.depth[step]);
  }

  /** The next step on a breakpoint line, or the end. */
  nextBreak(step: number, lines: ReadonlySet<number>): number {
    return this.find(step, 1, (j) => lines.has(this.line[j]));
  }

  /** The previous step on a breakpoint line, or the start. */
  prevBreak(step: number, lines: ReadonlySet<number>): number {
    return this.find(step, -1, (j) => lines.has(this.line[j]));
  }

  /** First step after `from` in direction `dir` matching `ok`; the end (or start) if none. */
  private find(from: number, dir: 1 | -1, ok: (j: number) => boolean): number {
    for (let j = from + dir; j >= 0 && j <= this.last; j += dir) if (ok(j)) return j;
    return dir === 1 ? this.last : 0;
  }
}
