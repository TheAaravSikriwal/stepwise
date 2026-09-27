// "What just happened": turns the change between two steps into plain
// English for someone who has never used a debugger. It only compares two
// replay states (plus the source and debug table to name things), so it
// works the same whichever direction you arrived from.

import type { DebugTable, Range } from "../compiler";
import type { FrameView, ReplayState } from "../replay/types";

export interface NarrationContext {
  debug: DebugTable;
  source: string;
  /** 1-based line number of an offset in the source. */
  lineOf(offset: number): number;
}

export interface Narration {
  /** What the last step did, one sentence each, in the order it happened. */
  happened: string[];
  /** What comes next, or `null` at the end. */
  next: string | null;
}

const code = (text: string) => `\`${text}\``;

export function narrate(
  prev: ReplayState | null,
  cur: ReplayState,
  ctx: NarrationContext,
  endMessage: string | null = null,
): Narration {
  const happened: string[] = [];

  if (prev === null) {
    happened.push("Your program is ready to start. Nothing has run yet.");
  } else {
    happened.push(...returns(prev, cur));
    happened.push(...variableChanges(prev, cur));
    for (const text of cur.output.slice(prev.output.length)) happened.push(`Printed ${text}.`);
    happened.push(...calls(prev, cur));
    const checked = condition(prev, cur, ctx);
    if (checked) happened.unshift(checked);
    if (happened.length === 0 && prev.line !== null) {
      happened.push(`Ran line ${lineOfStep(prev.line, ctx)}. Nothing you can see changed.`);
    }
  }

  if (cur.line === null) {
    happened.push(endMessage ? `The program stopped: ${endMessage}` : "The program finished.");
    return { happened, next: null };
  }
  const where = cur.frames.length > 1 ? ` in ${code(cur.frames[cur.frames.length - 1].name)}` : "";
  return { happened, next: `Next up: line ${lineOfStep(cur.line, ctx)}${where}, highlighted in yellow.` };
}

function lineOfStep(step: number, ctx: NarrationContext): number {
  const span = ctx.debug.steps[step];
  return span ? ctx.lineOf(span.from) : 0;
}

/** Functions that finished during the step, innermost first. */
function returns(prev: ReplayState, cur: ReplayState): string[] {
  const out: string[] = [];
  for (let i = prev.frames.length - 1; i >= cur.frames.length; i--) {
    const done = prev.frames[i].name;
    const caller = prev.frames[i - 1]?.name;
    // `main` finishing is the program finishing, which is said at the end.
    if (caller) out.push(`${code(done)} finished and handed its answer back to ${code(caller)}.`);
  }
  return out;
}

/** Functions that started during the step, outermost first. */
function calls(prev: ReplayState, cur: ReplayState): string[] {
  const out: string[] = [];
  for (let i = Math.max(prev.frames.length, 1); i < cur.frames.length; i++) {
    const f = cur.frames[i];
    const args = f.vars.map((v) => `${v.name} = ${v.display}`).join(", ");
    out.push(args ? `Called ${code(f.name)} with ${args}.` : `Called ${code(f.name)}.`);
  }
  return out;
}

/** New, changed and vanished variables in the frames that exist before and after. */
function variableChanges(prev: ReplayState, cur: ReplayState): string[] {
  const out: string[] = [];
  const shared = Math.min(prev.frames.length, cur.frames.length);
  for (let i = 0; i < shared; i++) {
    const before = byId(prev.frames[i]);
    const after = byId(cur.frames[i]);
    for (const v of cur.frames[i].vars) {
      const old = before.get(v.varId);
      if (!old) out.push(`Made a new variable ${code(v.name)} and set it to ${v.display}.`);
      else if (old.value !== v.value) out.push(`${code(v.name)} changed from ${old.display} to ${v.display}.`);
    }
    const gone = prev.frames[i].vars.filter((v) => !after.has(v.varId)).map((v) => code(v.name));
    if (gone.length === 1) out.push(`${gone[0]} went away, because the block it was made in ended.`);
    if (gone.length > 1) out.push(`${gone.join(", ")} went away, because the block they were made in ended.`);
  }
  return out;
}

function byId(frame: FrameView) {
  return new Map(frame.vars.map((v) => [v.varId, v]));
}

/**
 * If the last step checked an `if` or `while` condition, says what it
 * decided. The trace doesn't record the condition's value, but where the
 * program went next tells us: into the block right after the condition
 * means true. Only when no function was called, since then we can't tell yet.
 */
function condition(prev: ReplayState, cur: ReplayState, ctx: NarrationContext): string | null {
  if (prev.line === null) return null;
  // A call inside the condition means we don't know its value yet. (At the
  // very end, `main` returning in the same step is fine: the check was false.)
  if (prev.frames.length !== cur.frames.length && cur.line !== null) return null;
  const cond = ctx.debug.steps[prev.line];
  if (!cond) return null;
  const text = ctx.source.slice(cond.from, cond.to);
  // Conditions are the only steps that aren't whole statements ending in `;`.
  if (text.trimEnd().endsWith(";")) return null;
  const keyword = /(while|if)\s*$/.exec(ctx.source.slice(Math.max(0, cond.from - 12), cond.from))?.[1];
  if (!keyword) return null;

  const fnId = prev.frames[prev.frames.length - 1].fnId;
  const block = firstBlockAfter(cond, fnId, ctx.debug);
  const next = cur.line === null ? null : ctx.debug.steps[cur.line];
  const wentIn = !!(block && next && next.from >= block.from && next.to <= block.to);

  const checked = `Checked ${code(text)}:`;
  if (keyword === "while") {
    return wentIn
      ? `${checked} it's true, so the loop runs its body.`
      : `${checked} it's false, so the loop is done.`;
  }
  return wentIn
    ? `${checked} it's true, so the code inside the \`if\` runs.`
    : `${checked} it's false, so the code inside the \`if\` is skipped.`;
}

/** The block that starts right after a condition: the `if`'s or `while`'s body. */
function firstBlockAfter(cond: Range, fnId: number, debug: DebugTable): Range | null {
  let best: Range | null = null;
  for (const s of debug.scopes) {
    if (s.fnId !== fnId || s.span.from < cond.to) continue;
    if (!best || s.span.from < best.from) best = s.span;
  }
  return best;
}
