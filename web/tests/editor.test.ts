import { StringStream } from "@codemirror/language";
import { describe, expect, it } from "vitest";
import type { Diagnostic } from "../src/compiler";
import { toLint } from "../src/editor/editor";
import { startState, token } from "../src/editor/language";

/**
 * Runs the highlighter over some lines (sharing state, as the editor does),
 * returning `[text, style]` pairs with whitespace dropped.
 */
function highlight(...lines: string[]): [string, string | null][] {
  const state = startState();
  const out: [string, string | null][] = [];
  for (const line of lines) {
    const stream = new StringStream(line, 4, 4);
    while (!stream.eol()) {
      stream.start = stream.pos;
      const style = token(stream, state);
      const text = stream.current();
      if (text.trim()) out.push([text, style]);
    }
  }
  return out;
}

const styleOf = (pairs: [string, string | null][], text: string) => pairs.filter(([t]) => t === text).map(([, s]) => s);

describe("highlighter", () => {
  it("colours a function signature by kind", () => {
    expect(highlight("fn factorial(n: int) -> int {")).toEqual([
      ["fn", "definitionKeyword"],
      ["factorial", "functionDef"],
      ["(", "bracket"],
      ["n", "param"],
      [":", "punctuation"],
      ["int", "typeName"],
      [")", "bracket"],
      ["->", "operator"],
      ["int", "typeName"],
      ["{", "bracket"],
    ]);
  });

  it("keeps colouring parameters inside their function, and calls as calls", () => {
    const pairs = highlight("fn f(n: int) -> int {", "    return n * f(n - 1);", "}");
    expect(styleOf(pairs, "n")).toEqual(["param", "param", "param"]);
    expect(styleOf(pairs, "f")).toEqual(["functionDef", "function"]);
    expect(styleOf(pairs, "return")).toEqual(["controlKeyword"]);
  });

  it("forgets parameters when the next function starts", () => {
    const pairs = highlight("fn a(n: int) { }", "fn main() {", "    let n = 1;", "    print(n);", "}");
    expect(styleOf(pairs, "n")).toEqual(["param", "variableDef", "variableName"]);
  });

  it("marks where a variable is made, with or without mut", () => {
    expect(highlight("let mut total = 0;").slice(0, 3)).toEqual([
      ["let", "definitionKeyword"],
      ["mut", "definitionKeyword"],
      ["total", "variableDef"],
    ]);
  });

  it("handles comments, literals, builtins and bad characters", () => {
    expect(highlight("print(true && 42) # // done")).toEqual([
      ["print", "builtin"],
      ["(", "bracket"],
      ["true", "bool"],
      ["&&", "operator"],
      ["42", "number"],
      [")", "bracket"],
      ["#", "invalid"],
      ["// done", "comment"],
    ]);
  });

  it("never gets stuck on unusual characters", () => {
    expect(highlight("é😀&|").map(([, s]) => s)).toEqual(["invalid", "invalid", "invalid", "invalid", "invalid"]);
  });
});

describe("toLint", () => {
  const diag: Diagnostic = {
    severity: "error",
    message: "cannot assign twice to immutable variable `x`",
    span: { from: 30, to: 35 },
    labels: [{ span: { from: 10, to: 11 }, message: "declared here without `mut`" }],
    notes: ["make it mutable: `let mut x = 1;`"],
    rendered: "",
  };

  it("keeps the span and folds labels and notes into the message", () => {
    expect(toLint([diag], 100)).toEqual([
      {
        from: 30,
        to: 35,
        severity: "error",
        message:
          "cannot assign twice to immutable variable `x`\ndeclared here without `mut`\nnote: make it mutable: `let mut x = 1;`",
      },
    ]);
  });

  it("clamps spans to the document, since the text may have changed since compiling", () => {
    const [d] = toLint([diag], 32);
    expect([d.from, d.to]).toEqual([30, 32]);
  });
});
