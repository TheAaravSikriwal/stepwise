// Syntax highlighting for Stepwise.
//
// This is a small hand-written tokenizer just for colors. It is deliberately
// separate from the real lexer in Rust: highlighting has to be instant and
// forgiving on every keystroke, while the real lexer only runs when you press
// Run. Keep the keyword list in sync with compiler/src/lexer.rs.

import { HighlightStyle, StreamLanguage, syntaxHighlighting, type StringStream } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

export const KEYWORDS = new Set(["fn", "let", "mut", "if", "else", "while", "return"]);
export const LITERALS = new Set(["true", "false"]);
export const TYPES = new Set(["int", "bool"]);
export const BUILTINS = new Set(["print"]);

/** Returns the highlight token name for the text at the stream's position. */
export function token(stream: StringStream): string | null {
  if (stream.eatSpace()) return null;
  if (stream.match("//")) {
    stream.skipToEnd();
    return "comment";
  }
  if (stream.match(/^[0-9]+/)) return "number";
  const word = stream.match(/^[A-Za-z_][A-Za-z0-9_]*/) as RegExpMatchArray | null;
  if (word) {
    const w = word[0];
    if (KEYWORDS.has(w)) return "keyword";
    if (LITERALS.has(w)) return "bool";
    if (TYPES.has(w)) return "typeName";
    if (BUILTINS.has(w)) return "standard";
    // A name followed by `(` is a function call or definition.
    if (stream.match(/^\s*\(/, false)) return "function";
    return "variableName";
  }
  if (stream.match(/^(->|==|!=|<=|>=|&&|\|\||[+\-*/%=<>!])/)) return "operator";
  if (stream.match(/^[(){}]/)) return "bracket";
  if (stream.match(/^[,;:]/)) return "punctuation";
  stream.next();
  return "invalid";
}

export const stepwiseLanguage = StreamLanguage.define<null>({
  name: "stepwise",
  startState: () => null,
  token,
  languageData: { commentTokens: { line: "//" } },
  tokenTable: {
    function: t.function(t.variableName),
    standard: t.standard(t.variableName),
  },
});

// Colors come from CSS variables so light and dark mode both work (style.css).
const style = HighlightStyle.define([
  { tag: t.keyword, color: "var(--syn-keyword)", fontWeight: "600" },
  { tag: [t.number, t.bool], color: "var(--syn-number)" },
  { tag: t.comment, color: "var(--syn-comment)", fontStyle: "italic" },
  { tag: t.typeName, color: "var(--syn-type)" },
  { tag: [t.function(t.variableName), t.standard(t.variableName)], color: "var(--syn-function)" },
  { tag: t.operator, color: "var(--syn-operator)" },
  { tag: t.invalid, color: "var(--error)", textDecoration: "underline wavy" },
]);

export const stepwise = [stepwiseLanguage, syntaxHighlighting(style)];
