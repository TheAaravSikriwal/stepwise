// The replay engine: turns a recorded trace into the program's state at any
// step, forward or backward.
//
// Spec: docs/specs/replay.md. Tests: tests/replay.test.ts (correctness) and
// tests/replay-perf.test.ts (speed on a million-event trace).
//
// The design is "snapshots only" (option (b) in the spec):
//
//   - Building the replay applies the whole trace once, from the start, and
//     saves a full copy of the state every SNAPSHOT_EVERY steps.
//   - Seeking to step k loads the nearest snapshot at or before k and
//     applies events forward from there: at most SNAPSHOT_EVERY steps' worth.
//     Moving forward from where we already are skips the load.
//
// Backward is the same code path as forward, so there's no undo logic to get
// subtly wrong. The cost is memory for the snapshots, which is small here:
// a snapshot is the call stack and its variables, and the output is shared
// (it only ever grows, so a snapshot just remembers how long it was).

import type { DebugTable } from "../compiler";
import { Kind, type TraceData, printed } from "../runtime/trace";
import { type FrameView, type Replay, type ReplayState, varKey } from "./types";

const SNAPSHOT_EVERY = 1000;

interface Var {
  varId: number;
  value: number;
}

interface Frame {
  fnId: number;
  /** span_id of the last `line` event in this frame, or null. */
  line: number | null;
  /** Live variables, in declaration order. */
  vars: Var[];
}

/** Everything needed to continue applying events from some point. */
interface Snapshot {
  /** Index of the next event to apply. */
  next: number;
  frames: Frame[];
  /** How many lines had been printed. */
  printed: number;
}

const copyFrames = (frames: Frame[]): Frame[] =>
  frames.map((f) => ({ fnId: f.fnId, line: f.line, vars: f.vars.map((v) => ({ ...v })) }));

export function createReplay(trace: TraceData, debug: DebugTable): Replay {
  const { lines } = trace;
  const stepCount = lines.length + 1;

  // Every printed line, in order. Stepping only changes how many are shown.
  const output: string[] = [];
  for (let i = 0; i < trace.length; i++) {
    if (trace.kind[i] === Kind.Print) output.push(printed(trace, i));
  }

  /** How many events step k has applied: through its `line` event, or all of them at the end. */
  const eventsThrough = (k: number) => (k < lines.length ? lines[k] + 1 : trace.length);

  // ------------------------------------------------------ the current state

  let frames: Frame[] = [];
  let next = 0;
  let shown = 0;
  let step = 0;

  function apply(i: number): void {
    const a = trace.a[i];
    const top = frames[frames.length - 1];
    switch (trace.kind[i]) {
      case Kind.Call:
        frames.push({ fnId: a, line: null, vars: [] });
        break;
      case Kind.Line:
        if (top) top.line = a;
        break;
      case Kind.Declare:
        top?.vars.push({ varId: a, value: trace.b[i] });
        break;
      case Kind.Assign: {
        const v = top?.vars.find((x) => x.varId === a);
        if (v) v.value = trace.c[i];
        break;
      }
      case Kind.ScopeExit:
        if (top) top.vars = top.vars.filter((v) => debug.vars[v.varId]?.scopeId !== a);
        break;
      case Kind.Ret:
        frames.pop();
        break;
      case Kind.Print:
        shown++;
        break;
    }
  }

  function applyUntil(target: number): void {
    while (next < target) apply(next++);
  }

  function load(s: Snapshot): void {
    frames = copyFrames(s.frames);
    next = s.next;
    shown = s.printed;
  }

  // ------------------------------------------------------------ snapshots

  const snapshots: Snapshot[] = [];
  for (let k = 0; k < stepCount; k++) {
    if (k % SNAPSHOT_EVERY === 0) snapshots.push({ next, frames: copyFrames(frames), printed: shown });
    applyUntil(eventsThrough(k));
  }
  load(snapshots[0]);
  applyUntil(eventsThrough(0));

  function seek(k: number): void {
    const target = Math.max(0, Math.min(stepCount - 1, Math.trunc(k) || 0));
    const events = eventsThrough(target);
    const snapshot = snapshots[Math.floor(target / SNAPSHOT_EVERY)];
    // Load a snapshot when going backward, or when it's further along than
    // where we are; otherwise just keep applying from here.
    if (events < next || snapshot.next > next) load(snapshot);
    applyUntil(events);
    step = target;
  }

  // ----------------------------------------------------------- the answer

  /**
   * Variables declared or assigned by the events of the last step (after
   * the previous step's `line`, through this one's), keyed by frame index.
   * Walked backward from the current state, so the frame depth at each
   * event is known without replaying; only variables still live count.
   */
  function changed(): Set<string> {
    const from = step === 0 ? 0 : lines[step - 1] + 1;
    const to = eventsThrough(step);
    const out = new Set<string>();
    let depth = frames.length;
    for (let i = to - 1; i >= from; i--) {
      const kind = trace.kind[i];
      if (kind === Kind.Declare || kind === Kind.Assign) {
        const frame = frames[depth - 1];
        if (frame?.vars.some((v) => v.varId === trace.a[i])) out.add(varKey(depth - 1, trace.a[i]));
      } else if (kind === Kind.Call) {
        depth--;
      } else if (kind === Kind.Ret) {
        depth++;
      }
    }
    return out;
  }

  function frameView(f: Frame): FrameView {
    return {
      fnId: f.fnId,
      name: debug.functions[f.fnId]?.name ?? `function ${f.fnId}`,
      line: f.line,
      vars: f.vars.map((v) => {
        const info = debug.vars[v.varId];
        return {
          varId: v.varId,
          name: info?.name ?? `var ${v.varId}`,
          value: v.value,
          display: info?.ty === "bool" ? String(v.value !== 0) : String(v.value),
        };
      }),
    };
  }

  return {
    stepCount,
    seek,
    state(): ReplayState {
      return {
        step,
        line: step < lines.length ? trace.a[lines[step]] : null,
        frames: frames.map(frameView),
        output: output.slice(0, shown),
        changed: changed(),
      };
    },
  };
}
