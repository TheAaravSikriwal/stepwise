// The replay engine's interface. **Shared contract (draft).** You implement
// it in replay.ts; the debugger UI only ever talks to it through this.
// Spec: docs/specs/replay.md

/** The debugger's view of the program at one step. */
export interface ReplayState {
  /** Which step this is, `0 <= step < stepCount`. */
  step: number;
  /** `span_id` of the statement about to run (index into `debug.steps`), or `null` at the end. */
  line: number | null;
  /** The call stack, outermost (`main`) first, innermost last. Empty at the end. */
  frames: FrameView[];
  /** Everything printed so far. */
  output: string[];
  /** Variables declared or assigned during the previous step, as `varKey(frameIndex, varId)`. */
  changed: Set<string>;
}

export interface FrameView {
  /**
   * Different for every call, even of the same function at the same depth
   * (`f(1) + f(2)` makes two), so a frame can be told apart from a new one
   * that took its place.
   */
  callId: number;
  fnId: number;
  /** Function name, from the debug table. */
  name: string;
  /**
   * `span_id` of the last statement that started in this frame: the current
   * line for the innermost frame, the call site for outer frames. `null`
   * before the frame has run any statement.
   */
  line: number | null;
  /** Live variables, in the order they were declared. */
  vars: VarView[];
}

export interface VarView {
  varId: number;
  name: string;
  value: number;
  /** How to show the value: `"42"`, `"true"`. */
  display: string;
}

export interface Replay {
  /** Number of steps: one per `line` event, plus a final "program finished" step. */
  readonly stepCount: number;
  /** Moves to `step` (clamped to `0..stepCount-1`). Must be fast for any step, in either direction. */
  seek(step: number): void;
  /** The state at the current step. */
  state(): ReplayState;
}

export function varKey(frameIndex: number, varId: number): string {
  return `${frameIndex}:${varId}`;
}
