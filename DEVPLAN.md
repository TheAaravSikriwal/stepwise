# Stepwise: Development Plan

> Companion to `rewind-project-plan.md` (the *product* plan). This document is the *build* plan:
> who writes what, in what order, how we check it works, and what we do when things go wrong.
> Project name: **Stepwise**. Source files use the `.step` extension.

---

## 1. Purpose: why we're building this

The goal has three parts. When two of them conflict, the one higher on the list wins.

1. **You come out able to explain every core line.** The project is meant to be a portfolio and
   interview piece. A compiler you can't explain is worth less than a smaller one you can. That's
   why you write the core and I write the plumbing.
2. **It actually helps beginners.** Every feature has to answer the question "does this help a
   student see what their code is doing?" The debugger is the product. The language only exists
   to feed it.
3. **It ships.** A working MVP beats a perfect half-finished 1.0. Scope stays fixed; the only
   thing we cut is polish.

**What "done" means for this build stretch:** we keep going phase by phase until you run out of
credits. There are no planned stops. Every session ends with the repo in a state that can be
picked up again (section 7.4).

---

## 2. Who writes what

**Changed on 2026-09-27: I (Claude) build everything, core included.** At the start the core was
assigned to you, and everything below was prepared that way: a spec and a failing test suite for
each piece. You expected the whole thing to be built, so I now write the core against those specs
and tests. The interview goal still holds, so the core is written to be read: one idea per
function, comments that explain *why*, and a walkthrough of each piece whenever you want one.

| Area | Owner |
|---|---|
| Lexer, parser, checker, codegen and instrumentation, replay engine | Me, against `docs/specs/` and the existing tests |
| Everything else (plumbing, UI, tests, CI, docs) | Me, already built |
| Testing the finished app, and deciding when it goes on wearechintu.com | **You** |

## 2a. The core build plan

**Goal:** your "tour" program (and every gallery example) runs in the playground, can be stepped
through in both directions, and gets a plain-English explanation of every step. Then, and only
then, it's ready for you to test for the website.

**Order.** Each stage builds on the one before, and each ends at a hard gate:

| # | Piece | Gate (all must pass) | Then |
|---|---|---|---|
| 1 | Lexer (`lexer.rs`) | `--test lexer` (29) | `PIPELINE = Lexer` |
| 2 | Parser (`parser.rs`) | `--test parser --test parser_depth` (47) | `PIPELINE = Parser` |
| 3 | Checker (`checker.rs`) | `--test checker` (58); review every message in `--test error_messages` (15) before accepting the snapshots | `PIPELINE = Checker` |
| 4 | Codegen + instrumentation (`codegen.rs`) | `--test codegen --test examples --test phase0`; the web golden test (22 programs in the browser runtime) | `PIPELINE = Codegen` |
| 5 | Replay engine (`replay.ts`) | `tests/replay` (18 correctness + 4 speed) | stepping switches on |
| 6 | Integration | everything below | the release checklist |

**Stage 6: done means all of these:**
- Every test in both repos passes, and every CI step is **required** (no "in progress" steps left).
- `cargo clippy -D warnings` and `cargo fmt --check` are clean.
- The tour program is added as a golden test with its exact expected output.
- It's checked by hand in the browser: every gallery example runs and steps, forward and backward;
  the explainer reads correctly on recursion, loops and short-circuiting; breakpoints, step over
  and step out work; broken programs give the right messages; runaway loops and division by zero
  stop cleanly.
- The docs match reality: how-it-works describes the replay design as actually built, STATUS and
  CHANGELOG are current, and the stub and its "compiler unfinished" code paths are removed.

**Design choices, made now so they don't drift:**
- The lexer works on bytes, decoding a `char` only for non-ASCII error reporting.
- The parser is recursive descent, one function per grammar rule, with panic-mode recovery at
  statement boundaries, the missing-`;` special case, and a depth limit of 200.
- The checker makes two passes (signatures, then bodies). A scope stack maps names to `VarId`s. An
  internal error type (`Option<Type>`) stops cascades. "Every path returns" is a small separate
  function.
- Codegen uses one `FnCompiler` per function, with `local.tee` for old values, `scope_exit` for every
  open block on `return`, short-circuit `&&`/`||`, and `unreachable` at the end of a function that
  returns a value.
- The replay engine is **snapshots only** (option (b) in §3.2): a full copy of the state every 1,000
  steps, and every seek loads the nearest earlier snapshot and applies events forward. It has one
  code path for both directions, so there's less to go wrong. The speed tests prove it's fast enough.

**Risks and what to do about them:**
- *A test turns out wrong once real code exists.* Every suite was validated against a throwaway
  implementation, so this is unlikely. If it happens, fix the test and record why in the commit.
- *The wasm build behaves differently from native.* The web golden test runs the same 22 programs
  through the browser runtime.
- *Error messages read badly.* Read all 15 snapshots before accepting them, and fix the wording in
  the checker rather than accepting it as it is.
- *The session ends mid-stage.* Commit after each stage passes, and keep STATUS.md current.

---

## 3. Contracts decided up front

These are the interfaces between your code and mine. Fixing them now lets both of us work at the
same time without blocking each other.

### 3.1 The compiler API

```rust
pub fn compile(source: &str) -> CompileOutput;

pub struct CompileOutput {
    pub wasm: Option<Vec<u8>>,        // None if there were errors
    pub debug: DebugTable,
    pub diagnostics: Vec<Diagnostic>, // errors and warnings, all of them, not just the first
}
```

- **Spans are byte offsets** (`start..end`) into the source, inside Rust.
- The `compiler-wasm` wrapper converts spans to **line + UTF-16 column** for JavaScript, because
  CodeMirror counts in UTF-16. The core code never has to think about this.
- Source text is normalized to `\n` line endings before lexing. On Windows, pasted text is often
  `\r\n`, which would put every column after it off by one.

### 3.2 The trace ABI (improved over the product plan, section 5.2)

The compiled program imports these host functions from module `"sw"`. Every value is an `i32`.

| Import | Emitted when | Replay meaning |
|---|---|---|
| `line(span_id)` | a statement starts, **and every time a `while` condition is evaluated** | step boundary |
| `declare(var_id, value)` | `let` / `let mut` runs, and when a parameter binds on function entry | variable enters scope (there's no "old" value) |
| `assign(var_id, old, new)` | a reassignment runs | value change, which can be undone |
| `call(fn_id)` | a function is entered (the parameters follow as `declare` events) | push a frame |
| `ret(value)` | a function returns | pop a frame; undoing it must restore the popped frame |
| `scope_exit(scope_id)` | a block ends (on every path, including `return`) | variables leave scope |
| `print(value)` / `print_bool(value)` | `print(...)` runs (codegen picks one by the argument's type) | output line, stored in the same event stream so it rewinds too |

The authoritative list lives in `compiler/src/abi.rs`, and `web/src/runtime/run.ts` must match it.

These fix five gaps in the original design: parameter values were never recorded, undoing a
return had no data to restore, variables appeared before they existed and lingered after their
block ended, loop conditions weren't steps, and output wasn't guaranteed to line up with steps.

**Trace storage:** a struct of arrays, with one growable `Int32Array` each for `kind`, `a`, `b`
and `c`, plus a separate index of which events are `line` events (the steps). It's filled in the
worker and handed to the main thread as transferable buffers, so nothing gets copied.

**The replay engine interface** (TypeScript; you implement it and I build the UI against it):

```ts
interface Replay {
  readonly stepCount: number;
  seek(step: number): void;          // must be fast for any step
  state(): {
    line: SpanId;
    frames: Frame[];                  // innermost last
    output: string[];
    changed: Set<VarKey>;             // variables changed by the most recent step
  };
}
```

**One decision we'll make at Phase 6, not now:** undoing a `ret` needs the popped frame. There are
two options:

- (a) remember popped frames while applying events forward, and if a frame isn't available,
  rebuild from the nearest snapshot;
- (b) make stepping backward always mean "load the nearest earlier snapshot and apply events
  forward".

(b) is simpler; (a) is the better interview story. Either way the tests are identical, and I'll
set up a cross-check that runs both approaches against each other. The trace format doesn't
change whichever we pick.

### 3.3 The debug table

`functions[fn_id] = { name, span }`, `vars[var_id] = { name, fn_id, scope_id, span }`,
`spans[span_id] = { start, end }`. I define the Rust type and its conversion to JSON; your
resolver and codegen fill it in.

---

## 4. Tools and defaults (chosen by me; object to any of them)

- **Rust** stable with `wasm-encoder`, `wasmparser` and `insta`. `wasmtime` is a dev-dependency
  so golden tests run under `cargo test` alone, with no JavaScript needed while you work on the
  compiler.
- **Web:** TypeScript + Vite, **no UI framework**. The UI is a handful of panels, and plain DOM
  keeps the dependency count and the amount to learn down. Vitest for web tests. CodeMirror 6.
- A small dev CLI, `stepc`: `stepc file.step --tokens | --ast | --wat | --run`. Being able to see each
  stage's output is the single best debugging tool for a compiler.
- `.gitattributes` forces LF line endings, so snapshot tests give the same result on Windows and
  on the Linux CI runners.
- **Git:** a local repo from day one, with small commits. Creating a GitHub remote and deploying
  are public actions, so I'll ask before doing either.

---

## 5. Build order: two tracks in parallel

Your track is always the core phase from the product plan. My track keeps the plumbing a step
ahead, so that when your piece works it plugs straight into something real.

| Step | My track | Your track | Exit check |
|---|---|---|---|
| **0** | Repo scaffold, Cargo workspace, Vite app, `compiler-wasm`, worker. A stub `compile()` returns a hand-built module (made with `wasm-encoder`) that prints 42 | (Nothing: read *Crafting Interpreters* ch. 4, "Scanning") | Browser shows `42`, coming from a worker. CI workflow runs locally |
| **1a** | `span.rs`, `Diagnostic` and its pretty renderer, **lexer spec + failing tests**, `stepc --tokens` | — | `cargo test` shows the lexer tests failing for the right reason (`todo!()`) |
| **1b** | Editor: CodeMirror, highlighting, error underlines from diagnostics, console panel, Run button, worker protocol | **Lexer** | Lexer tests pass; I review it |
| **1c** | Parser spec, AST types (we agree on them together, since they're shared), parser tests, `stepc --ast` | **Parser** | Example program → correct AST; parse errors have line and column |
| **2** | Resolver/type-checker spec and error-message snapshot tests. Error rendering in the editor | **Resolver + type checker** | Type errors underlined in the browser before anything runs |
| **3** | Golden-program harness (wasmtime), `wasmparser` validation on every module, `stepc --wat`, 15+ golden programs | **Codegen** (without trace calls) | All golden programs print the right output |
| **4** | Runtime host functions, trace buffer, step limit, trap → friendly-message mapping. **Playground end-to-end** | **Instrumentation** (the trace calls from 3.2) | Type, Run, see output or errors. Traces are recorded |
| **5** | Debugger UI: current-line highlight, variables panel with change highlights, call stack, time-synced output, keyboard controls. Built against a fake `Replay` first | **Replay engine, forward** | Stepping forward shows correct variables and call stack |
| **6** | Replay property harness: full forward = real run, full undo = initial state, and it reports the **first step where they diverge** | **Replay backward** | Both properties hold for every golden program → **MVP 0.1** |
| 7+ | Timeline scrubber, breakpoints, "where did this value come from?", arrays, share links, gallery, CI deploy | Snapshots, reverse watchpoint, array codegen | As in the product plan |

For every core piece, I deliver three things in `docs/specs/` **before** you start it: a one-page
spec (what it does, its interface, edge cases, reading pointers), the failing tests, and a review
checklist.

---

## 6. Problems we can already see coming, and what to do about each

| Problem | How we'll notice | What to do |
|---|---|---|
| **Invalid WASM** from codegen | `wasmparser` fails in tests | `stepc --wat` on the smallest failing program; the validator error points at the bad instruction |
| **Wrong answer** from a compiled program | A golden test fails | Shrink the program to the smallest one that still fails, keep it as a new golden test, then compare `--ast` and `--wat` output |
| **Wrong line highlighted** | Line-highlight golden tests | Find the pipeline stage that lost the span. Spans must never be dropped (product plan, 5.1) |
| **Replay differs from the real run** | Property harness | It reports the first step and event where they diverge; debug from there, never from the end |
| **Infinite loop** | Event counter hits the limit | The host function throws, the partial trace is kept, and the UI says it ran too long and asks whether a loop never ends |
| **Recursion too deep** | JS `RangeError` from the WASM stack | Catch it and report "recursion too deep" at the last line reached. The partial trace stays debuggable |
| **Division by zero**, or `INT_MIN / -1` | WASM trap | Map the trap to plain English at the last `line` event |
| **Huge traces** | Memory checks on long programs | Growable typed arrays, the event limit, and snapshots in Phase 7 |
| **Byte vs UTF-16 column mismatch** | A test with a non-ASCII comment | Convert only in the `compiler-wasm` layer (3.1) |
| **CRLF line endings** | A test with CRLF input | Normalize to `\n` before lexing |
| **Windows toolchain issues** (PATH, `wasm-pack`, linker) | Build fails | Check the MSVC Build Tools, then `rustup show`, and pin the versions in `rust-toolchain.toml` |
| **You're stuck on a core piece** | More than ~45 minutes without progress | The hint ladder (7.2) |
| **Scope creep** | "Wouldn't it be cool if…" | Write the idea in `IDEAS.md` and don't build it until 1.0 ships |
| **Credits run out mid-task** | — | Commit early and often; `STATUS.md` is always current (7.4) |
| **The AST needs to change after I've built on it** | Your parser wants a different shape | Fine: the AST is shared, so we agree on the change and I update the plumbing |

---

## 7. How we work together

### 7.1 The loop for each core piece
1. I write the spec and failing tests, and you read them. Ask anything; changing the spec is allowed.
2. You write the code and run `cargo test` (or `npm test`) until everything passes.
3. You tell me it's done, and I review it against the checklist: correctness, spans preserved,
   error quality, and "could you explain this line in an interview?"
4. You fix the review comments, we commit, and I plug your piece into the plumbing.

### 7.2 The hint ladder (when you're stuck)
Ask for one level at a time:
1. **Concept:** what idea is missing ("precedence climbing handles this by…").
2. **Structure:** the shape of the solution (which functions, what they return).
3. **Pseudocode:** the steps, but not Rust.
4. **Code:** only if you explicitly ask, and then you type it yourself and we walk through it.

### 7.3 Reviews I'll do even when all the tests pass
- Any spot where a span gets lost or made up
- Error messages that aren't plain English or don't point at the right place
- `unwrap()` / `panic!` on user input (the compiler must never crash on bad code)
- Code you'd have trouble explaining

### 7.4 End-of-session checklist (so any new session can pick up where we left off)
- All tests pass, or failing tests are clearly marked as your current work
- Committed
- `STATUS.md`: where we are, what's next on each track, open questions
- `DEVLOG.md`: a few lines about what happened and what was hard (material for the blog post)

---

## 8. Open questions (none of them block starting)

- Hosting: GitHub Pages or Cloudflare Pages. This only matters from Phase 4 on.
- Integer overflow: wrap (the product plan says so for v1) or trap with a friendly error. I'd lean
  toward trapping, since wrapping confuses beginners; decide at Phase 3.
- Option (a) or (b) for stepping backward (3.2): decide at Phase 6.

---

## 9. The name

**Decided: Stepwise.** Folder `stepwise/`, crates `stepwise-compiler` / `stepwise-wasm`, CLI `stepc`, files `*.step`, host import module `"sw"`.
extension and the folder name. See the options in chat.

---

## 10. Plan for v0.2: the Electric look, "where did this value come from?", and the call galaxy

Decided 2026-09-28. Build in this order; each stage is tested, checked in the browser, and
committed before the next.

### Stage A: the Electric look
- **Theme:** GitHub-dark style (page #0d1117, panels #161b22, borders #30363d), cobalt/blue for
  actions, yellow (#ffd100) for "now" (the current line and just-changed values). A **light
  version** follows the OS setting and has a toggle that's remembered. `?theme=dark` still forces
  dark for the wearechintu.com frame.
- **Nothing sharp:** rounded cards with gaps between them instead of hard dividing lines, pill
  buttons and menus, soft transitions (hover, the changed-value flash, menus opening).
- **Colour by kind of code**, using a highlighter that tracks context:
  - declaration keywords (`fn`, `let`, `mut`) and control-flow keywords (`if`, `else`, `while`,
    `return`) each get their own colour;
  - function names get one colour where they're defined (bold) and another where they're called;
  - parameters (recognised in the signature, then everywhere in that function's body);
  - a variable being made (after `let`);
  - types, numbers, `true`/`false`, the built-in `print`, operators and comments.
- **Done when:** the gallery, tour, errors, stops and menus all look right in dark, light and the
  ~960px frame, and every existing test still passes.

### Stage B: "Where did this value come from?"
- Each variable in the Variables panel gets a "where from?" button. It jumps to the moment that
  value was set, flashes the line that set it, and the explainer says what happened ("`total` was
  set on line 113: it changed from 14 to 30").
- **How:** the replay engine records which call each event belongs to, so it can scan back from
  the current step for the last `declare`/`assign` of that variable in that same call. This is
  exact even when recursion has many copies of the same variable.
- **Tests:** unit tests on the fixtures (the same variable in different calls, a variable set
  before a loop, parameters set at the call), and a check on the tour in the browser.

### Stage C: the call galaxy (3D) (done 2026-09-27)
- **As built:** `web/src/galaxy/` (`model.ts` the tree and layout, `view.ts` the three.js scene,
  `panel.ts` the tab). Calls past the cap fold into their nearest kept caller; the tooltip and the
  legend say how many, rather than a "×N" label. Only `main`, the current call and the first call
  of each recursion are named, so deep recursion doesn't bury the view in labels.
- A **3D view of the run**: every function call is a bubble, and the calls it makes cluster around
  it (recursion becomes a chain). Each call's variables are small bubbles orbiting it. Size means
  time spent in that call, and the call running now glows yellow. Drag to look around, scroll to
  zoom, click a bubble to jump to that moment. It grows and shrinks as you move through the run.
- **How:** three.js, loaded only when the galaxy is opened. A *call tree* built once from the
  trace (the calls, their parents, first and last step, steps spent) sets out a stable 3D layout,
  so bubbles don't jump around while scrubbing. Each step only shows or hides bubbles and moves the
  glow.
- **Limits:** a run can have thousands of calls, so past a cap (~2,000 bubbles) repeated calls
  collapse into one bubble labelled "×N".
- **Tests:** unit tests for the call tree (nesting, step ranges, counts, collapsing), and checks
  in the browser for readability, speed while scrubbing, and dark and light themes.

### Stage D: black, white and gold (done 2026-09-27)
Asked for after the galaxy: "no clear boundaries, everything floating in space", and a look that
feels expensive.
- **Colours:** black (#060607), warm white (#f4f1ea) and gold (#d4af37); an ivory light theme
  with deeper gold. Gold means "now" (and marks Run); white outlines where a value came from; red
  is still a problem.
- **Structure:** no cards or borders. The name top left, the two views as big buttons top centre,
  examples and Share just above the code. Panel names in gold capitals over a fading gold rule, so
  it's clear what's under each.
- **Galaxy view:** the map fills the page behind everything; the panels turn to frosted glass
  over it, so the map under them blurs and the values stay the thing you read. The camera centres
  the map in the open space left of the panels (`GalaxyView.setFocus`).
- **Variables:** "`fib`'s variables at step 85", with Name and Value headings.
- **Galaxy colours:** each function gets a clear colour, jewel tones rich enough to sit with the
  black and gold (sapphire, emerald, ruby, amethyst, teal, rose, azure, jade); `main` is ivory.
  Gold and anything near it is kept for the call running now.
