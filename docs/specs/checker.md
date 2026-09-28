# Spec: the checker (name resolution and type checking)

**Status:** built (2026-09-27) · **Files:** `compiler/src/checker.rs` (yours; split it into `resolve.rs` and
`types.rs` if you like), `compiler/src/checked.rs` (shared output types, draft)
**Tests:** `compiler/tests/checker.rs` · **Run:** `cargo test --test checker`
**Reading:** *Crafting Interpreters*, chapter 11 ("Resolving and Binding"). Type checking isn't
covered there, but it has the same shape: a walk over the tree.
**Prerequisite:** the parser passes its tests.

## What it does

It walks the tree, works out what every name refers to, and checks that the types fit together.
This is where most beginner mistakes get caught, and each one gets a clear message *before the
program runs*:

```text
error: cannot find a variable named `totl`
 --> main.step:4:11
  |
4 |     print(totl);
  |           ^^^^
  = note: did you mean `total`?
```

```rust
pub fn check(program: &Program) -> (Checked, Vec<Diagnostic>);
```

It only runs on programs that parsed without errors. `compile()` stops after parsing if there
were any, because checking a broken tree produces confusing errors.

## Output: side tables

Codegen needs to know, for each name in the tree, which variable or function it means, plus the
type of each expression. Rather than changing the AST, `Checked` holds maps **keyed by the span
of the node** (see `checked.rs`):

| Map | Key | Value |
|---|---|---|
| `vars` | span of every identifier naming a variable: `let` names, parameter names, uses, assignment targets | `VarId` |
| `calls` | span of the callee name in every call | `Callee::Function(id)` or `Callee::Print(arg_type)` |
| `scopes` | span of every `Block` | `ScopeId` |
| `types` | span of every expression | `Type` |
| `functions` | index = position in `program.functions` | `FnSig` |

**Numbering.** IDs go up by one in the order things appear in the source. Functions: `FnId`
0, 1, 2, … in file order. Variables: `VarId`s are numbered across the whole program, in order of
declaration; within a function, parameters come first, then `let`s. Scopes: `ScopeId`s are also
numbered program-wide, in the order the blocks are entered.

**Debug table** (`checked.debug`): fill `functions` (name, and the span of the name), `vars` (name,
type, `fn_id`, `scope_id`, name span) and `scopes` (`fn_id`, parent scope or `None` for a function
body, block span). Leave `steps` empty; that's codegen's job. The debugger uses this table to show
names and values, so it has to line up exactly with the side tables.

## Rules

### Names and scopes
1. **Two passes.** First, collect every function's signature. Then check the bodies. That's what
   lets functions call functions defined later, and themselves.
2. A variable can be used from its declaration onward, in its block and any nested blocks.
   Parameters belong to the function's body scope.
3. `let x = x + 1;`: the `x` on the right is *not* the new `x` (it isn't declared until the
   statement finishes). That makes it an error, unless an outer `x` exists, which rule 4 forbids
   anyway.
4. **No shadowing, and no redeclaring.** Declaring a name that's already visible (from the same
   scope, an outer scope, or a parameter) is an error, with a label on the first declaration. Sibling
   blocks can reuse names. (This is stricter than Rust: two live variables with the same name would
   confuse beginners in the variables panel.)
5. **`let` and parameters are immutable.** Assigning to one is an error. Label the declaration
   and suggest `let mut` (for a parameter, suggest copying it into a `let mut`).
6. **Unknown names:** "cannot find a variable named `x`". If a visible variable is within edit
   distance 2 of the name, add the note ``did you mean `total`?``.
7. **Using a function as a variable** (`let x = f;`), or **calling a variable** (`x()`), is its
   own error that says which it is.

### Types
8. The types are `int` and `bool`. Any other type name is an error, with a note listing both.
9. Operators:

   | Operator | Operands | Result |
   |---|---|---|
   | `+ - * / %` | int, int | int |
   | `< <= > >=` | int, int | bool |
   | `== !=` | both int or both bool | bool |
   | `&& \|\|` | bool, bool | bool |
   | `-` (unary) | int | int |
   | `!` | bool | bool |

   Point the error at **the operand that has the wrong type**, and say what was expected and what
   was found.
10. `if` and `while` conditions must be `bool`. When the condition is an `int`, add the note
    ``if you meant "is not zero", write `x != 0` ``. The test checks for `!= 0`.
11. `let x: T = e;` requires `e` to have type `T`. Without an annotation, `x` gets `e`'s type.
    Assignment must keep the variable's type.

### Calls and returns
12. Calls check the argument count (say how many were expected) and each argument's type (point
    at the argument).
13. `print` is built in and takes exactly one `int` or `bool`. Defining a function named `print`
    is an error. Record the argument type in `Callee::Print` so codegen can pick `print` or
    `print_bool`.
14. A call to a function with no `-> type` has type `Unit`. You can use it as a statement, but
    storing it, passing it or printing it is an error ("`f` doesn't return a value").
15. `return e;` must match the declared return type. `return;` is only allowed in functions with
    no return type, and `return e;` only in functions that have one.
16. **Every path must return** in a function with a return type. A statement list *always
    returns* if any statement in it always returns: a `return` does, and an `if` does when **both**
    branches always return. A `while` never counts (it might run zero times). Report the error at
    the function name.

### The program
17. There must be a `fn main()` with no parameters and no return type.
18. Two functions can't share a name (label the first), and two parameters of one function can't
    either.

### No cascades
19. **One mistake, one error.** When an expression has an error, give it a special internal
    "error type" that's silently compatible with everything. Then `let x = y + 1;` with an unknown
    `y` reports `y` once, and every later use of `x` stays quiet. Likewise, an unknown function's
    call result is error-typed. Don't put the error type in `checked.rs`; keep it inside your
    checker (for example `Option<Type>`, or your own enum).

## Hints

<details><summary>Hint 1: the scope stack</summary>

Keep a stack of scopes, where each scope maps names to `VarId`s. Entering a block pushes a
scope, leaving pops it. Looking a name up searches from the top of the stack down. For rule 4,
"is this name visible" is the same search.
</details>

<details><summary>Hint 2: one function per node kind</summary>

`check_stmt(&Stmt)` and `check_expr(&Expr) -> Option<Type>` (with `None` as the error type from
rule 19). `check_expr` records `types[expr.span]` on the way out. Binary operators become a
`match` on the operator that checks both operand types.
</details>

<details><summary>Hint 3: "always returns"</summary>

Write `fn always_returns(stmts: &[Stmt]) -> bool` as a small recursive function, separate from
the main walk. It's easier to test and to explain.
</details>

<details><summary>Hint 4: edit distance</summary>

Levenshtein distance fits in about 15 lines using a single `Vec<usize>` row. Only compare
against visible variables, and only suggest when the distance is 1 or 2 and the name is longer
than 2 characters.
</details>

## Review checklist
- [ ] `cargo test --test checker` passes
- [ ] The side tables and the debug table agree (the `debug_*` tests check this)
- [ ] No cascading errors (the `cascade_*` tests)
- [ ] Error messages read well for a beginner. Try some in the playground once it's wired up
- [ ] You can explain why there are two passes, and why the error type prevents cascades
- [ ] Run `cargo test --test error_messages`, then `cargo insta review`: read all 15 messages in
      `compiler/tests/errors/` as a beginner would before accepting them
