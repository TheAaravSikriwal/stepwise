# Spec: the parser

**Owner:** you · **Files:** `compiler/src/parser.rs` (yours), `compiler/src/ast.rs` (shared draft)
**Tests:** `compiler/tests/parser.rs`, `compiler/tests/parser_depth.rs`
**Run:** `cargo test --test parser --test parser_depth`
**Reading:** *Crafting Interpreters*, chapters 5 ("Representing Code"), 6 ("Parsing Expressions")
and 8 ("Statements and State")
**Prerequisite:** the lexer passes its tests.

## What it does

It turns the token list into a tree ([`ast.rs`](../../compiler/src/ast.rs)) that the later stages
walk. The parser checks **shape** only: `let x = y;` parses fine even if `y` doesn't exist. The
resolver catches that later.

```rust
pub fn parse(source: &str) -> (Program, Vec<Diagnostic>);
```

`parse` calls `lexer::lex` itself and returns the lexer's diagnostics along with its own. It
always returns a `Program` containing whatever parsed successfully, and it **never panics**.

## Grammar

`*` means zero or more, `?` means optional, and quoted text is a literal token.

```text
program    = function*
function   = "fn" IDENT "(" params? ")" ( "->" type )? block
params     = param ( "," param )*
param      = IDENT ":" type
type       = IDENT
block      = "{" stmt* "}"

stmt       = "let" "mut"? IDENT ( ":" type )? "=" expr ";"
           | IDENT "=" expr ";"                       -- assignment
           | "if" expr block ( "else" ( block | if_stmt ) )?
           | "while" expr block
           | "return" expr? ";"
           | expr ";"

expr       = or
or         = and ( "||" and )*
and        = comparison ( "&&" comparison )*
comparison = additive ( ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) additive )?   -- at most one!
additive   = term ( ( "+" | "-" ) term )*
term       = unary ( ( "*" | "/" | "%" ) unary )*
unary      = ( "-" | "!" ) unary | primary
primary    = INT | "true" | "false"
           | IDENT ( "(" args? ")" )?                  -- variable or call
           | "(" expr ")"
args       = expr ( "," expr )*
```

Notes:

- **Precedence**, loosest to tightest: `||`, `&&`, comparisons, `+ -`, `* / %`, unary. All
  binary operators are left-associative (`1 - 2 - 3` is `(1 - 2) - 3`).
- **Comparisons don't chain.** `1 < x < 5` is an error, not `(1 < x) < 5`. In most languages
  that compiles and silently does the wrong thing, which is a classic beginner trap. The error
  should suggest `1 < x && x < 5`.
- **Assignment vs expression:** a statement that starts with `IDENT` followed by `=` is an
  assignment. Look ahead one token to decide.
- **Negative literals are folded:** a `-` directly followed by an `INT` token becomes one
  `Int(-n)` expression spanning both tokens. That's the only way to write `-2147483648`. An `INT`
  of 2147483648 anywhere else is an error ("too large").
- **`else if`** becomes an else block whose only statement is the inner `if`. The block's span is
  the inner `if`'s span.
- The `if` and `while` conditions need no parentheses. `if (x) {}` also works, because `(x)` is
  just an expression.

## Spans (the tests check these)

| Node | Span |
|---|---|
| Expression | the whole expression: `a * (b + c)` covers from `a` to `)`; a call covers through its `)` |
| Statement | including its `;`. `if` and `while` cover through their last `}` (including `else`) |
| Function | from `fn` to its closing `}` |
| Block | from `{` to `}` |
| `Ident` | just the name |

## Errors and recovery

This is the hard part, and it's what makes the compiler feel friendly. The goal: **one mistake,
one error.** No cascade of confusing follow-on errors.

**Messages.** The tests check spans and key words, not exact wording:

| Situation | Span | Must mention |
|---|---|---|
| Missing `;` | **empty span right after the previous token** (not the start of the next line) | `` `;` `` |
| Missing `)` | at the unexpected token | `` `)` `` |
| Unclosed `{` at end of file | the unclosed `{`, as the primary span or as a label | `` `}` `` |
| No expression where one is needed (`let x = ;`) | the token found instead | — |
| `let` not followed by a name (`let 5 = x;`) | the token found instead | — |
| Chained comparison | the second comparison | `&&` (in a note) |
| `2147483648` without `-` | the literal | — |
| Top-level code that isn't a function | the first token | `fn`, e.g. "all code must be inside a function" |
| Nested too deeply | where the limit was reached | — |

**Recovery (panic mode).** When a statement fails to parse, record the error, then skip tokens
until a likely statement boundary: just past a `;`, or right before a `}` or a keyword that
starts a statement (`let`, `if`, `while`, `return`). Then carry on. At the top level, skip to
the next `fn`. *Crafting Interpreters* section 6.3 explains this in detail.

**The missing-semicolon special case.** When a `;` is missing and the next token is on a new
line, report the error and carry on *as if the `;` were there*, without skipping anything. That
way `let y = 2` followed by `print(y);` gives exactly one error.

**Depth limit.** The browser gives the compiler a small stack, and a stack overflow crashes
instead of producing an error. Count the nesting depth (increment when entering an expression or
block, decrement on the way out), and past a limit (**200** is plenty) report "this is nested too
deeply" and stop descending. `parser_depth.rs` checks this with 5000 levels.

## Hints (read only as far as you need)

<details><summary>Hint 1: the parser's state</summary>

A struct holding the token list, the index of the current token, the source (to get identifier
text from spans), and the diagnostics collected so far. A few small helpers carry most of the
work: `peek()` (look at the current token), `advance()` (consume it), `check(kind)`, and
`expect(kind, what) -> Option<Token>`, which reports an error if the token is wrong. Because the
token list always ends in `Eof`, `peek()` can never run off the end, as long as `advance()` never
moves past `Eof`.
</details>

<details><summary>Hint 2: one function per grammar rule</summary>

Each line of the grammar becomes a method: `parse_or` calls `parse_and` in a loop, `parse_and`
calls `parse_comparison`, and so on down to `parse_primary`. Looser rules call tighter ones.
That's all precedence is. The left-associative loop looks like: parse the left side; while the
current token is one of my operators, consume it, parse the right side, and build
`Binary { lhs: left, rhs: right }` as the new left.
</details>

<details><summary>Hint 3: handling errors with Option</summary>

Have the parse methods return `Option<Expr>` / `Option<Stmt>`, where `None` means "an error was
already reported". Then `?` propagates failure up to the statement level, and the statement
parser does the recovery (skip to a boundary) and continues. Only statement-level code decides
how to recover.
</details>

<details><summary>Hint 4: empty span after the previous token</summary>

Keep the end of the *previous* token around (for example in `advance()`, or by looking at
`tokens[pos - 1]`). For "expected `;`", report at `Span::new(prev_end, prev_end)`.
</details>

## Stretch goals (not tested)

- **Unclosed-brace hint from indentation:** when a `{` isn't closed, find the `}` whose
  indentation doesn't match its `{` and point there instead. It gives a better message for the
  most common mistake.
- **"Did you mean `==`?"** when `=` appears inside an `if` or `while` condition.

## Review checklist

- [ ] `cargo test --test parser --test parser_depth` all pass
- [ ] One function per grammar rule, and you can explain how precedence falls out of the call order
- [ ] No `unwrap()` on anything that depends on input
- [ ] Try a few typos in the playground (once it's wired in) and check the messages read well
- [ ] You can explain panic-mode recovery and why the missing-`;` case is special
