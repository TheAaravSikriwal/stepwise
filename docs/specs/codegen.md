# Spec: code generation and instrumentation

**Owner:** you · **Files:** `compiler/src/codegen.rs` (yours; `instrument.rs` too if you like)
**Tests:** `compiler/tests/codegen.rs` and the golden programs in `compiler/tests/programs/`
**Run:** `cargo test --test codegen` · **Try it:** `cargo run -p stepc -- wat docs/examples/factorial.step`
**Reading:** the `wasm-encoder` docs, `compiler/src/stub.rs` (a complete tiny module), and MDN's
"Understanding WebAssembly text format" (for reading `stepc wat` output)
**Prerequisite:** the checker passes its tests.

## What it does

It walks the checked tree and emits a WebAssembly module. The module does two things at once:
it **computes the program's result**, and it **calls the trace imports** at every step, so the
debugger can replay the run afterwards.

```rust
pub fn codegen(program: &Program, checked: &Checked) -> Output; // { wasm, steps }
```

Once it works, set `PIPELINE = Stage::Codegen` in `lib.rs`, delete `stub.rs`, and the
playground runs real programs.

## Module layout

| Part | Rule |
|---|---|
| Imports | Call `abi::emit_imports` **first**: 8 imports, types 0–7, function indices 0–7 |
| Functions | One wasm function per Stepwise function, in order: `FnId` *i* is wasm function `abi::IMPORT_COUNT + i` |
| Types | One type per function: `n` × `i32` params, one `i32` result unless the return type is `Unit` |
| Exports | Only `main` |
| Values | `int` is `i32`; `bool` is `i32` 0 or 1 |
| Locals | wasm params are the function's parameters, in order. Then add one `i32` local per other variable of that function, plus any scratch locals you need |

Map `VarId` → local index from `checked.debug.vars`: a function's variables are the ones with its
`fn_id`, and the parameters come first.

## Expressions

| Stepwise | WebAssembly |
|---|---|
| `5`, `true`/`false` | `i32.const 5`, `i32.const 1`/`0` |
| `x` | `local.get` |
| `-e` | `i32.const 0`, *e*, `i32.sub` |
| `!e` | *e*, `i32.eqz` |
| `+ - * / %` | `i32.add sub mul div_s rem_s` (division truncates toward zero; overflow wraps) |
| `< <= > >= == !=` | `i32.lt_s le_s gt_s ge_s eq ne` |
| `a && b` | **short-circuit:** *a*, `if (result i32)` *b* `else` `i32.const 0` `end` |
| `a \|\| b` | *a*, `if (result i32)` `i32.const 1` `else` *b* `end` |
| `f(args)` | args left to right, then `call` |
| `print(e)` | *e*, then `call` the `print` import, or `print_bool` if `Callee::Print(Type::Bool)` |

`&&` and `||` must short-circuit: the right side might call a function that prints (see
`short_circuit.step`).

## Statements and the trace they emit

This is the contract with the replay engine, so it has to be exact. The tests compare whole event
sequences. "*events of e*" means whatever evaluating `e` emits (for example, the calls inside it).

| Statement | Emits, in order |
|---|---|
| function entry | `call(fn_id)`, then `declare(var, value)` for each parameter in order |
| `let x = e;` | `line(s)` · *events of e* · `declare(x, value)` |
| `x = e;` | `line(s)` · *events of e* · `assign(x, old, new)` |
| `e;` | `line(s)` · *events of e* (plus `drop` the value if it isn't `Unit`) |
| `if c { A } else { B }` | `line(c)` · *events of c* · then the chosen block (or nothing) |
| `while c { B }` | per check: `line(c)` · *events of c*; when true, run `B` and check again |
| a block `{ ... }` | its statements, then `scope_exit(scope)` when it finishes normally |
| `return e;` | `line(s)` · *events of e* · `scope_exit` for **every** open block, innermost first, up to and including the function body · `ret(value)` |
| `return;` | `line(s)` · the same `scope_exit`s · `ret(0)` |
| end of a `Unit` function | (the body block's `scope_exit`) · `ret(0)` |

Here `s` is the statement's step and `c` is the condition's step.

**Step IDs.** Each `line` gets a new `span_id`: 0, 1, 2, … in **source order** (functions in
order, statements top to bottom, a condition before its blocks). `steps[span_id]` is the span to
highlight: the **whole statement** for `let`, assignment, expression statements and `return`, and
the **condition expression** for `if` and `while`.

**Why each rule exists** (you'll want these for interviews):
- The line event comes *before* the statement, so the highlighted line is the one "about to
  run". That's how every debugger behaves.
- The `while` condition gets its own step on every check, including the final false one, so
  beginners can watch the loop decide to stop.
- `assign` carries the old value, which makes undo a single store (product plan, §5.2). Read the
  local *before* evaluating the new value. `local.tee` is handy here.
- `scope_exit` on every path (including `return` from deep inside loops) is what stops the
  variables panel from showing dead variables.

**Ending a function that returns a value.** The checker guarantees every path returns, but wasm
validation doesn't know that. End such bodies with `unreachable`.

## Tests

- **Golden programs** (`compiler/tests/programs/*.step`): each has `// expect: <line>` comments
  for its expected output, or `// expect-trap: <text>` for a runtime error. The test compiles
  and runs every program under wasmtime and reports every mismatch at once. **Add a golden
  program every time you fix a bug.**
- **Trace checks:** every golden program's trace is checked for consistency: every `assign`'s
  `old` matches the variable's current value, variables are declared once per scope entry,
  every scope is exited before `ret`, calls and returns balance, and every `line` has a step.
  If this fails, the message says which event is wrong.
- **Exact traces:** a few small programs whose event sequences are spelled out in full.
- **Validity:** every module is checked with `wasmparser` before running.

## Hints

<details><summary>Hint 1: structure</summary>

A `FnCompiler` struct per function holds the `Function` being built, the `VarId`→local map, a
stack of open scope IDs (for the `return` rule), and a reference to a shared step list.
Functions: `stmt(&Stmt)`, `expr(&Expr)`, `block(&Block)`. Emit the imports and types first, then
compile each function, then assemble the sections (see `stub.rs` for the order).
</details>

<details><summary>Hint 2: while loops in wasm</summary>

```text
block            ;; br 1 from inside the loop jumps past this: loop exit
  loop           ;; br 0 jumps back here: loop start
    (line c) c
    i32.eqz
    br_if 1      ;; condition false, so exit
    <body>       ;; ends with its scope_exit
    br 0
  end
end
```
</details>

<details><summary>Hint 3: reading your output</summary>

`cargo run -p stepc -- wat file.step` prints the module as text. When a golden test fails,
shrink the program until the problem is obvious, and read the `wat`.
</details>

## Review checklist
- [ ] `cargo test --test codegen` passes: golden outputs, trace checks, exact traces
- [ ] `stepc wat` output for `factorial.step` is readable, and you can walk through it
- [ ] You can explain short-circuiting, `local.tee` for old values, and the `scope_exit`s on `return`
- [ ] Bump `PIPELINE` to `Codegen` and try the playground. Later, delete `stub.rs` and the
      `Stub`-stage branches in `lib.rs`. `tests/phase0.rs` should keep passing unchanged, because
      real codegen emits exactly the trace the stub fakes.
