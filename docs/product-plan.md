# Rewind: A Browser Playground with a Step-Backward Debugger

> **Working title.** "Rewind" is a placeholder name. Pick your own before launch.

---

## 1. The Idea in One Paragraph

Rewind is a website where anyone can write code in a small, purpose-built programming language and run it instantly in the browser, with nothing to install. Every run is recorded, so the user can step through the program line by line, watch variables change, and, most importantly, **step backward** to see exactly what happened before something went wrong. It is aimed at people learning to program, who often struggle to picture what their code is doing while it runs. Under the hood it is a real compiler written in Rust that turns source code into WebAssembly, plus a replay engine that makes time-travel debugging feel instant.

---

## 2. The Problem and the Audience

**The problem.** Beginners write a loop or a recursive function, get the wrong answer, and have no idea where it went wrong. Traditional debuggers only move forward: if you step past the moment a variable went bad, you have to restart and try again. Print statements help, but they force the learner to guess in advance what to print.

**The audience.**

- **Primary:** students in intro programming courses (starting with Northeastern), who need to build a mental model of how code executes.
- **Secondary:** TAs and instructors who want a zero-setup tool to demonstrate loops, recursion, and scope in class or office hours.
- **Tertiary:** curious engineers, who will visit because a step-backward debugger in a browser is unusual. These people share links.

**Why it's different.** Reverse debugging exists in professional tools like `rr`, but it is rare to find it in a browser with zero setup, and rarer still in a tool designed for learners. The language is deliberately small so the debugger is the star.

---

## 3. Features

### MVP (version 0.1)

The MVP is the smallest version that shows off the core idea.

- **Editor** with syntax highlighting and line numbers.
- **Compile and run** in the browser, with output shown in a console panel.
- **Clear error messages** that underline the exact problem in the code and explain it in plain English.
- **Forward and backward stepping**, one line at a time.
- **Variables panel** showing the current value of every variable in scope, with values that just changed highlighted.
- **Call stack panel** showing which function called which, so recursion is visible.
- **Output synced to time:** as you step backward, printed output disappears in step with it.
- **Step limit:** infinite loops stop cleanly with a message instead of freezing the tab.

### Version 1.0

- **Timeline scrubber:** drag a slider to jump anywhere in the run.
- **Step over / step out / run to breakpoint**, in both directions.
- **"Where did this value come from?":** click any variable and jump straight to the step where it was last assigned. This is a reverse watchpoint, and it's cheap to build because the whole run is already recorded. It's likely to be the most memorable feature, so give it prominence.
- **Arrays and strings**, with an array view that highlights which index just changed.
- **Shareable links:** the program is encoded in the URL, so a TA can send a link to a prepared example.
- **Example gallery:** factorial, Fibonacci, bubble sort, binary search, a buggy program to find, and so on.

### Later (stretch goals)

- A memory view showing linear memory as a grid.
- Guided "find the bug" exercises.
- A Python-like syntax mode, if user feedback asks for it.
- Embedding: an `<iframe>` snippet instructors can drop into course pages.

### Explicitly out of scope for version 1

Keeping these out is what keeps the project finishable: classes and objects, closures, garbage collection, floating-point numbers, user input while a program runs, modules and imports, and a backend server of any kind.

---

## 4. The Language (Version 1 Spec)

Keep it small, strict, and readable. A beginner should be able to learn the whole language in ten minutes.

**Types:** `int` (32-bit), `bool`. Arrays and strings arrive in version 1.0.

**Statements:** `let` (immutable), `let mut` (mutable), assignment, `if` / `else`, `while`, `return`, expression statements.

**Functions:** named, with typed parameters and return types; recursion allowed. Execution starts at `main`.

**Built-ins:** `print(value)`.

**Example program:**

```
fn factorial(n: int) -> int {
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
```

**Design notes.**

- 32-bit integers keep the boundary between WebAssembly and JavaScript simple, because JavaScript numbers hold 32-bit values exactly. Overflow wraps in version 1; consider trapping with a friendly error later, since wrapping surprises beginners.
- Static typing is worth the extra work: it catches mistakes before the program runs, and good type errors are a teaching feature in themselves.
- Write a short language reference page (one screen long) as part of the site.

---

## 5. Architecture

The system has two halves: a **compiler** (Rust, compiled to WebAssembly so it runs in the browser) and a **runtime + debugger** (TypeScript). There is no server; the whole site is static files.

```
 ┌──────────────┐    source    ┌─────────────────────────────────────────────┐
 │    Editor    │ ───────────▶ │ Compiler (Rust → WASM, in a Web Worker)      │
 │ (CodeMirror) │              │ lexer → parser → type checker → codegen      │
 └──────────────┘              │                  + instrumentation           │
        ▲                      └───────────────────────┬─────────────────────┘
        │ errors / highlights                          │ program.wasm + source map
        │                                              ▼
 ┌──────┴───────┐    trace     ┌─────────────────────────────────────────────┐
 │ Debugger UI  │ ◀─────────── │ Runtime (Web Worker)                        │
 │ vars, stack, │              │ runs program.wasm, host functions record    │
 │ output, time │              │ every event into a trace buffer             │
 └──────────────┘              └─────────────────────────────────────────────┘
        ▲
        │ state at step N
 ┌──────┴───────┐
 │ Replay engine│  applies / undoes trace events, uses snapshots to jump
 └──────────────┘
```

### 5.1 The compiler pipeline

1. **Lexer:** turns source text into tokens (`fn`, `factorial`, `(`, …). Every token carries its **span**, meaning its start and end position in the source. Spans are what make good error messages and line highlighting possible, so never drop them.
2. **Parser:** turns tokens into an abstract syntax tree (AST). Use a hand-written recursive-descent parser with precedence climbing for expressions. It's easier to debug than a parser generator and produces better errors.
3. **Name resolution and type checking:** resolves each variable to its declaration, assigns each local a numeric ID, checks types, and checks that `let` variables are never reassigned. Collect multiple errors rather than stopping at the first one.
4. **Code generation:** walks the checked AST and emits WebAssembly using the `wasm-encoder` crate. Each function becomes a WASM function; each local variable becomes a WASM local.
5. **Instrumentation** (done during code generation): inserts calls to host functions that record what the program is doing (see 5.2).
6. **Output:** the `.wasm` bytes, plus a **debug table** mapping span IDs to source positions, variable IDs to names, and function IDs to names.

### 5.2 Record, then replay (the key design decision)

WebAssembly can't easily be paused in the middle of running, so Rewind does not try to execute backward. Instead:

1. The compiler inserts small calls into the program at every important moment.
2. The program runs to completion (usually in milliseconds), and every call writes an event into a trace.
3. The debugger lets the user scrub through that recording in both directions.

To the user it feels exactly like pausing and rewinding. Several real time-travel debuggers use the same approach, so say so openly in the write-up.

**Host functions the compiled program imports:**

| Function | Called when | Records |
|---|---|---|
| `trace_line(span_id)` | a new statement starts | which line is executing |
| `trace_assign(var_id, old, new)` | a variable is assigned | old and new value |
| `trace_call(fn_id, arg_count)` | a function is entered | new stack frame |
| `trace_return(value)` | a function returns | frame removed, return value |
| `trace_store(addr, old, new)` | an array element is written (v1.0) | old and new value |
| `print(value)` | the program prints | output text |

Recording the **old** value at assignment time is what makes stepping backward cheap: undoing an assignment just restores the old value. The compiler emits a read of the variable before the write so the old value is available.

**The trace format.** Store events in typed arrays (e.g. an `Int32Array` per field) rather than JavaScript objects. A run can produce millions of events, and typed arrays keep that fast and memory-efficient.

**Steps.** A "step" is the span between two `trace_line` events. Stepping forward applies events until the next line marker; stepping backward undoes events back to the previous one.

**Snapshots.** Undoing one event at a time is fine for single steps but slow for jumping from step 900,000 to step 12. Every 1,000 steps, save a full snapshot of the state (all live frames and their variables). To jump anywhere, load the nearest earlier snapshot and apply events forward from there.

**The step limit.** Each host function increments a counter. When it passes the limit (start with about 1 million events), the host function throws, which stops the WASM program immediately. The debugger then shows the partial recording and a message: "Stopped after 1,000,000 steps — is there an infinite loop?" The user can still debug the recording to find the loop.

**Run in a Web Worker.** Both compiling and running happen in a Web Worker, so even a slow program never freezes the page.

### 5.3 The debugger UI

- The editor highlights the current line, based on the span of the current step.
- The variables panel shows the reconstructed state at the current step; values that changed during the last step are highlighted.
- The call stack panel lists frames, with the innermost on top.
- The output panel shows only what had been printed by the current step.
- Controls: step back, step forward, (v1.0) step over, step out, run to breakpoint in either direction, and the timeline scrubber.
- Keyboard shortcuts for every control (e.g. arrow keys for stepping). This matters for accessibility and makes the demo feel fast.

---

## 6. Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Compiler | Rust | Fast, safe, and compiles cleanly to WebAssembly |
| WASM output | `wasm-encoder` crate | Builds valid WASM without writing raw bytes |
| WASM validation (tests) | `wasmparser` crate | Confirms every generated module is valid |
| Rust → browser | `wasm-pack` + `wasm-bindgen` | Standard toolchain for shipping Rust to the web |
| Frontend | TypeScript + Vite | Fast builds; a light framework like Svelte or Preact is optional |
| Editor | CodeMirror 6 | Lightweight, extensible, supports custom highlighting |
| Snapshot tests | `insta` crate | Easy golden-file tests for compiler output and errors |
| Hosting | Static hosting (Cloudflare Pages, Vercel, or GitHub Pages) | No backend needed, free |
| CI/CD | GitHub Actions | Test, build, and deploy on every push |

---

## 7. Repository Structure

```
rewind/
├── compiler/                 # Rust crate: the compiler itself
│   ├── src/
│   │   ├── lib.rs            # public entry point: compile(source) -> Result
│   │   ├── span.rs           # source positions
│   │   ├── lexer.rs
│   │   ├── ast.rs
│   │   ├── parser.rs
│   │   ├── resolve.rs        # name resolution, variable IDs
│   │   ├── types.rs          # type checker
│   │   ├── codegen.rs        # AST → WASM
│   │   ├── instrument.rs     # trace calls inserted during codegen
│   │   ├── debug_table.rs    # span/variable/function metadata
│   │   └── errors.rs         # error types + friendly formatting
│   └── tests/
│       ├── programs/         # .rw source files with expected output
│       └── errors/           # programs that should fail, with expected messages
├── compiler-wasm/            # thin wasm-bindgen wrapper exposing compile() to JS
├── web/
│   ├── src/
│   │   ├── editor/           # CodeMirror setup, highlighting, error underlines
│   │   ├── worker/           # Web Worker: compile + run
│   │   ├── runtime/          # host functions, trace buffer, step limit
│   │   ├── replay/           # replay engine, snapshots, state reconstruction
│   │   ├── ui/               # panels, controls, timeline
│   │   └── main.ts
│   ├── public/examples/      # example programs for the gallery
│   └── index.html
├── docs/
│   ├── language.md           # the one-page language reference
│   └── architecture.md       # how it works (becomes the blog post)
├── .github/workflows/        # CI: test, build, deploy
├── README.md
└── LICENSE
```

---

## 8. Testing Strategy

A compiler is exactly the kind of project where tests save you, because bugs are subtle and show up far from their cause.

- **Lexer and parser unit tests:** small inputs, expected tokens or AST shapes.
- **Golden program tests:** a folder of example programs, each with its expected output. The test harness compiles each one, runs it (using a WASM runtime such as `wasmtime` in Rust tests, or Node in JS tests), and compares output. Add a test every time you fix a bug.
- **Error message tests:** programs that should fail, with snapshot tests of the exact error text. This keeps error quality from quietly getting worse.
- **WASM validity:** run every generated module through `wasmparser`. Invalid WASM caught in tests is much easier to fix than a cryptic browser error.
- **Replay correctness (the most important test):** for every golden program, check two properties:
  1. Replaying the whole trace forward from the start reproduces the program's actual final state and output.
  2. Undoing the whole trace from the end returns exactly to the initial state.

  If both hold for every program, stepping in either direction is correct. Also test that jumping via snapshots gives the same state as stepping one at a time.
- **Later: fuzzing.** Generate random small valid programs and check the replay properties on them. This finds edge cases you'd never write by hand.

---

## 9. Build Plan

Hour estimates assume you write the core (parser, type checker, codegen, replay engine) yourself and use AI tools mainly for plumbing and UI. Each phase has an **exit criterion**, meaning a concrete thing that must work before moving on.

| # | Phase | Hours | Exit criterion |
|---|---|---|---|
| 0 | Setup and "hello world" pipeline | 4–6 | A hard-coded WASM module runs in the browser from a Web Worker and prints to the page |
| 1 | Lexer and parser | 15–25 | The example program parses into a correct AST; parse errors show a line and column |
| 2 | Name resolution and type checker | 15–25 | Type errors are caught before running, with underlined spans and plain-English messages |
| 3 | Code generation | 25–40 | All golden programs (arithmetic, `if`, `while`, recursion) compile, run, and print correct output |
| 4 | Playground | 10–20 | Type in the editor, press Run, see output or errors, all in the browser |
| 5 | Tracing and forward stepping | 15–25 | Every run is recorded; you can step forward line by line with correct variables and call stack |
| 6 | Stepping backward | 10–20 | Step backward works; both replay-correctness tests pass for every golden program |
| — | **MVP complete (≈ 60–80 hours)** | | **Soft-launch to friends (see section 10)** |
| 7 | Snapshots and timeline scrubber | 10–15 | Jumping anywhere in a long run is instant |
| 8 | "Where did this value come from?" and breakpoints | 8–12 | Click a variable to jump to its last assignment; breakpoints work in both directions |
| 9 | Arrays and strings | 15–25 | Array programs (e.g. bubble sort) work end to end, with an array view |
| 10 | Shareable links and example gallery | 5–8 | A link reproduces the exact program; gallery has 8–10 examples |
| 11 | Polish, docs, and launch | 15–25 | Everything on the launch checklist below is done |
| — | **Version 1.0 (≈ 120–200 hours total)** | | **Public launch** |

**Rough calendar.** At about 10 hours a week: MVP in 6–8 weeks, version 1.0 in 3–5 months. At about 20 hours a week: MVP in 3–4 weeks, version 1.0 in 6–10 weeks.

**Rules for staying on track.**

- Finish each phase's exit criterion before starting the next. A compiler built out of order is painful to debug.
- Resist adding language features. Every new feature touches the parser, type checker, codegen, instrumentation, and replay.
- Keep a simple dev log (a few lines per session). It becomes the raw material for your write-up and interview stories.

---

## 10. Release Process

Release in three stages, so real feedback shapes version 1.0 before strangers see it.

### Stage 1: Private alpha (after the MVP)

- Deploy to a preview URL.
- Give it to 3–5 friends or classmates. Watch at least two of them use it in person or on a call without helping them. Where they hesitate is your to-do list.
- Fix the top three confusions before moving on.

### Stage 2: Beta with real learners (during phases 7–10)

- Share with students in an intro course, and ask a TA or instructor whether they'd try it in office hours.
- Add a simple feedback link (a short form is fine).
- Track what people struggle with and which examples they open.

### Stage 3: Public launch (version 1.0)

**Launch checklist:**

- [ ] Deployed on its own subdomain with HTTPS
- [ ] Page loads fast on a normal laptop and on mobile (even if editing on mobile is limited)
- [ ] Clear landing experience: a pre-loaded example with a visible hint like "Press Run, then Step Back"
- [ ] Example gallery and one-page language reference
- [ ] "How it works" page explaining record-and-replay in plain English
- [ ] Shareable links tested across browsers (Chrome, Firefox, Safari)
- [ ] Social preview image and page title/description, so shared links look good
- [ ] README with a short GIF of stepping backward, setup instructions, architecture summary, and a clear list of what you wrote vs. what libraries you used
- [ ] Open-source license (MIT is a common choice)
- [ ] Issue templates for bug reports
- [ ] If you add analytics, use a privacy-friendly option and say what you collect on the site

**Where to share it:** your course communities and CS clubs, r/ProgrammingLanguages, r/compsci, Hacker News ("Show HN"), and LinkedIn. The blog post (below) usually travels further than the tool itself, so post both.

**Versioning.** Use semantic versioning (0.1 for the MVP, 1.0 for launch) and keep a `CHANGELOG.md`.

**CI/CD pipeline (GitHub Actions):**

1. On every push: run Rust tests, run web tests, build the compiler to WASM, build the site.
2. On pull requests: deploy a preview URL.
3. On merge to `main`: deploy to production.

---

## 11. Presenting It

### The write-up

Write a blog post titled something like "How I built a time-travel debugger that runs in your browser." Cover the record-and-replay design, how old values make undo cheap, how snapshots make jumping instant, and one hard bug you hit and how you found it. This post is often what gets a project noticed, and it prepares you for interview questions.

### Résumé bullet template

Fill in the real numbers once you have them:

> **Rewind** (Rust, WebAssembly, TypeScript): Built a compiler for a statically typed teaching language that runs entirely in the browser, with a step-backward debugger based on record-and-replay tracing; replays [X] million events with instant seeking via snapshots; used by [N] students.

### Numbers worth measuring

- Compile time for a typical program
- Events recorded per second, and the largest run you support
- Time to jump to any step using snapshots
- Number of users, shared links, or courses using it

---

## 12. Risks and How to Handle Them

| Risk | Mitigation |
|---|---|
| **Scope creep** (adding language features) | The out-of-scope list in section 3 is a hard rule for version 1 |
| **Source mapping bugs** (wrong line highlighted) | Carry spans through every stage; test line highlighting on golden programs |
| **Replay drifts from the real run** | The two replay-correctness tests run on every program in CI |
| **Huge traces use too much memory** | Typed arrays, the step limit, and snapshots; measure memory on long programs |
| **Burnout mid-project** | The MVP is useful and showable on its own; soft-launch it even if 1.0 takes longer |
| **Relying too much on AI for the core** | Write or line-by-line review the parser, type checker, codegen, and replay engine, so you can explain them in interviews |

---

## 13. Learning Resources

- ***Crafting Interpreters*** by Robert Nystrom (free online): the best practical guide to lexers, parsers, and language implementation.
- **The WebAssembly specification** and MDN's WebAssembly guides, for the instruction set and how modules work.
- **`wasm-encoder` and `wasm-bindgen` documentation**, for generating WASM in Rust and exposing Rust to JavaScript.
- **Write-ups on `rr` and other time-travel debuggers**, for how professional reverse debuggers approach recording and replay.
