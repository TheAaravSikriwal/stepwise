# Spec: the replay engine

**Owner:** you · **File:** `web/src/replay/replay.ts` (yours) · **Interface:** `web/src/replay/types.ts` (shared draft)
**Tests:** `web/tests/replay.test.ts` (correctness), `web/tests/replay-perf.test.ts` (speed)
**Run:** `cd web && npx vitest run tests/replay`
**Reading:** product plan §5.2 ("Record, then replay"); write-ups of `rr` and other time-travel debuggers
**Prerequisite:** none! The tests use hand-built traces, so you can write this before, after,
or while you work on the compiler.

## What it does

The program has already run, and the trace holds every event it produced (see
`web/src/runtime/trace.ts`). The replay engine turns that recording into **the program's state at
any step**: the current line, the call stack, each frame's variables, and the output so far. The
debugger UI calls `seek(step)` and then `state()`, and never looks at the trace directly.

```ts
createReplay(trace: TraceData, debug: DebugTable): Replay
```

## Steps

- There is one step per `line` event, **plus one final "finished" step**, so
  `stepCount = trace.lines.length + 1`.
- **Step `k < lines.length`** is the moment *just before* the statement of the `k`-th `line`
  event runs. Its state is the result of applying every event up to **and including** that
  `line` event. `state().line` is that event's `span_id`.
- **The final step** applies every event. `line` is `null`, the stack is empty (for a completed
  run), and `output` is everything printed.
- `seek` clamps to `0..stepCount-1`. A new replay starts at step 0.

## Applying events

| Event | Effect |
|---|---|
| `call(fn)` | push a frame for `fn` (name from `debug.functions`), with no variables and `line = null` |
| `declare(v, x)` | add variable `v` = `x` to the top frame (appended: declaration order) |
| `assign(v, old, new)` | set `v` to `new` in the top frame |
| `scope_exit(s)` | remove from the top frame every variable whose `debug.vars[v].scopeId == s` |
| `ret(x)` | pop the top frame |
| `line(s)` | set the top frame's `line` to `s` |
| `print(x, isBool)` | append the formatted value to the output (use `printed()` from `trace.ts`) |

`display` is `String(value)` for `int`, and `"true"` / `"false"` for `bool` (the type is in
`debug.vars[v].ty`).

**`changed`** holds the variables declared or assigned by the events *between the previous
step's `line` event (exclusive) and this step's (inclusive)*, as `varKey(frameIndex, varId)`
where `frameIndex` 0 is `main`. Only include variables still live at this step. At step 0, the
range starts at the first event.

`state()` must return a **snapshot**: later seeks mustn't change a state object the UI is
holding (the tests check this).

## The two properties that matter

1. **Correctness:** the state at a step doesn't depend on how you got there. Seeking forward,
   backward, or jumping at random must give exactly what seeking directly on a fresh replay
   gives. The tests check this on every fixture.
2. **Speed:** on a trace of about a million events (the event limit), random jumps average under
   5 ms and single steps are near-instant. `replay-perf.test.ts` checks this with generous
   budgets.

## How to get there (a suggested path)

**Milestone 1: correct and slow.** `seek(k)` rebuilds the state from nothing by applying events
`0..lines[k]`. It's tiny, obviously correct, and passes `replay.test.ts`. Commit it!

**Milestone 2: fast forward.** If the target is ahead of the current step, apply only the events
in between.

**Milestone 3: fast backward, which is the interesting part.** You have two options (DEVPLAN §3.2):

- **(a) Undo.** Reverse each event: `assign` restores `old` (that's why the trace records it),
  `declare` removes the variable, `call` pops, `line` restores the previous line. The hard ones:
  - `ret` must bring back the popped frame, and `scope_exit` must bring back removed variables.
    The trace doesn't hold them, so remember them as you apply those events forward (for
    example, a map from event index to the removed frame or variables).
  - After a jump, you may reach an event you never applied forward, so you have nothing
    remembered. Then fall back to rebuilding from a snapshot.
- **(b) Snapshots only.** Every N steps (1,000 is a good start), store a copy of the full state.
  Going backward means loading the nearest snapshot at or before the target and applying events
  forward. It's simpler, with one code path for everything.

Either way, **snapshots make long jumps fast**, so build them on the first full pass in
`createReplay`. A good interview answer can explain why (b) is enough here, and when (a) is worth
it (a much bigger state, where copying it every N steps would cost too much memory).

## Hints

<details><summary>Hint 1: state representation</summary>

Keep a mutable "current state": an array of frames, each with `fnId`, `line`, and an ordered
list of `{ varId, value }`, plus the output lines and the event index you're at. `state()` copies
it into the `ReplayState` shape (that's the snapshot requirement).
</details>

<details><summary>Hint 2: computing `changed`</summary>

It depends only on the events between two consecutive `line` events, so compute it from that
event range directly rather than tracking it while you seek. Then it's automatically the same
whichever direction you arrived from.
</details>

<details><summary>Hint 3: snapshot memory</summary>

A snapshot is a deep copy of the frames plus the output *length* (output only ever grows, so
you can keep one output array and slice it). 500 snapshots of a small state is nothing.
</details>

## Review checklist
- [ ] `npx vitest run tests/replay` passes (both files)
- [ ] You can explain why recording `old` in `assign` makes undo cheap, and why snapshots make jumps fast
- [ ] You can say which of (a) and (b) you chose, and why
- [ ] Play with the debugger on a recursive program and step back through the returns
