# Spec: the lexer

**Owner:** you · **File:** `compiler/src/lexer.rs` · **Tests:** `compiler/tests/lexer.rs`
**Run:** `cargo test --test lexer` · **Reading:** *Crafting Interpreters*, chapter 4 ("Scanning")

## What it does

It turns source text into a flat list of tokens, each with the exact byte span it came from.

```text
let mut i = 1;
```
becomes
```text
Let 0..3   Mut 4..7   Ident 8..9   Eq 10..11   Int(1) 12..13   Semicolon 13..14   Eof 14..14
```

The lexer knows nothing about grammar: `let let let` lexes fine, and the parser rejects it later.

## Interface

```rust
pub fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>);
```

- `source` is already newline-normalized (no `\r`). You don't need to handle `\r\n`, but you must
  not panic if a `\r` shows up; treating it as an unexpected character is fine.
- Always return a list ending in exactly one `Eof` token, whose span is `len..len`.
- **Never panic.** Any string is valid input, including emoji, lone `&` and giant numbers. Every
  problem becomes a `Diagnostic`.
- After an error, skip the offending text and keep lexing, so that one typo gives one error.

## Tokens

| Source | Token |
|---|---|
| `fn let mut if else while return true false` | keyword tokens |
| `[A-Za-z_][A-Za-z0-9_]*`, other than a keyword | `Ident` |
| `[0-9]+` | `Int(value)` |
| `( ) { } , ; :` | `LParen RParen LBrace RBrace Comma Semicolon Colon` |
| `->` | `Arrow` |
| `+ - * / %` | `Plus Minus Star Slash Percent` |
| `= == != < <= > >=` | `Eq EqEq BangEq Lt LtEq Gt GtEq` |
| `&& \|\| !` | `AndAnd OrOr Bang` |

**Rules**

1. **Longest match wins.** `==` is `EqEq`, not two `Eq`s. `===` is `EqEq Eq`.
2. **Keywords must match the whole word.** `letter` is an `Ident`. Keywords are case-sensitive.
3. **`int`, `bool` and `print` are identifiers, not keywords.** Types and built-ins are names that
   the resolver looks up, which keeps the lexer small and lets arrays and strings (v1.0) add types
   without touching it.
4. **Whitespace** (space, tab, newline) separates tokens and is otherwise ignored.
5. **Comments** start with `//` and run to the end of the line. Anything can appear inside a
   comment, including non-ASCII characters.
6. **Negative numbers aren't tokens.** `-5` is `Minus Int(5)`. The parser handles unary minus.
7. **Integer limit:** an integer literal can be at most **2147483648**. That's one more than the
   largest `int`, so that `-2147483648` can be written. The parser will reject `2147483648`
   without a minus in front. Anything larger is a lexer error.

## Errors

Write the messages in plain English for a beginner. The tests check the span and a few key
words, not your exact wording, so the phrasing is up to you.

| Input | Span | Must mention | Suggested wording |
|---|---|---|---|
| unknown character, e.g. `#`, `é`, `😀` | the whole character | the character itself | ``unexpected character `#` `` |
| lone `&` | the `&` | `&&` (in the message or a note) | ``unexpected `&` `` + note ``for "and", use `&&` `` |
| lone `\|` | the `\|` | `\|\|` | same idea |
| number over the limit | all of its digits | "too large" or "too big" | `this number is too large for an int (the largest is 2147483647)` |

Don't emit a token for text that caused an error. Just skip it.

## Edge cases the tests check

- The empty source gives just `Eof 0..0`.
- A comment at the very end of the file, with no newline after it.
- `a-->b` is `Ident Minus Arrow Ident`.
- Leading zeros are fine: `007` is `Int(7)`.
- **Multi-byte characters.** `é` is 2 bytes and `😀` is 4. An error span must cover the whole
  character. If you step through the source one byte at a time and slice mid-character, Rust will
  panic, and the random-input test is there to catch exactly that.
- 2000 random inputs must satisfy these invariants: spans are in bounds, in order and on character
  boundaries, only `Eof` has an empty span, and there's no panic.

## Hints (read only as far as you need)

<details><summary>Hint 1: the overall shape</summary>

Keep a position in the source and loop until you reach the end. At each position, look at the
current character, decide what kind of token *starts* there, consume as many characters as that
token needs, and push it. Each token's span is the position when you started it to the position
when you finished.
</details>

<details><summary>Hint 2: bytes or chars?</summary>

Every token except errors is ASCII, so working with bytes (`source.as_bytes()`) is simple and
fast. Only the "unexpected character" case needs to know how long a character is. When you hit a
non-ASCII byte, decode the full `char` at that position (`source[pos..].chars().next()`) and skip
`ch.len_utf8()` bytes.
</details>

<details><summary>Hint 3: two-character operators</summary>

Write a small helper that says "if the next byte is X, consume it and return true". Then `=` is:
if the next byte is `=`, it's `EqEq`, otherwise it's `Eq`.
</details>

<details><summary>Hint 4: numbers too large</summary>

Accumulate into a `u64` (or use `checked_mul` / `checked_add`) so a 30-digit number can't
overflow while you're reading it. Consume *all* the digits first, then check against the limit,
so the error covers the whole number.
</details>

## Review checklist (what I'll look at)

- [ ] Every test passes, including `robust_never_panics_on_random_input`
- [ ] No `unwrap()` / indexing that could panic on unusual input
- [ ] Error messages are friendly, and the spans point at exactly the bad text
- [ ] Keyword lookup is simple (a `match` on the word is ideal)
- [ ] You can explain why `Eof` exists and why error recovery matters to the parser
- [ ] `stepc tokens docs/examples/factorial.step` output looks right to you

## Trying it by hand

```bash
cargo run -p stepc -- tokens docs/examples/factorial.step
```
