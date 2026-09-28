// Syntax highlighting for Stepwise: every kind of code gets its own colour.
//
// A small hand-written tokenizer, separate from the real lexer in Rust:
// highlighting has to be instant and forgiving on every keystroke, while the
// real lexer only runs when you press Run. It keeps a little context between
// tokens, so it can tell apart
//   - declaration keywords (`fn`, `let`, `mut`) from control flow (`if`, `while`, ...),
//   - a function being defined from a function being called,
//   - parameters (seen in the signature, then everywhere in that function),
//   - a variable being made (right after `let`) from one being used.
// Keep the keyword lists in sync with compiler/src/lexer.rs.

import { HighlightStyle, StreamLanguage, type StringStream, syntaxHighlighting } from "@codemirror/language";
import { Tag, tags as t } from "@lezer/highlight";

export const DECLARATIONS = new Set(["fn", "let", "mut"]);
export const CONTROL = new Set(["if", "else", "while", "return"]);
export const LITERALS = new Set(["true", "false"]);
export const TYPES = new Set(["int", "bool"]);
export const BUILTINS = new Set(["print"]);

export interface HighlightState {
  /** What the next name will be, from the words just before it. */
  expect: "anything" | "function name" | "parameters" | "variable name";
  /** Inside a function's `( ... )` parameter list. */
  inParams: boolean;
  /** The current function's parameters, so their uses are coloured too. */
  params: string[];
}

export const startState = (): HighlightState => ({ expect: "anything", inParams: false, params: [] });

const copyState = (s: HighlightState): HighlightState => ({ ...s, params: [...s.params] });

/** Returns the highlight token name for the text at the stream's position. */
export function token(stream: StringStream, state: HighlightState): string | null {
  if (stream.eatSpace()) return null;
  if (stream.match("//")) {
    stream.skipToEnd();
    return "comment";
  }
  if (stream.match(/^[0-9]+/)) return "number";

  const match = stream.match(/^[A-Za-z_][A-Za-z0-9_]*/) as RegExpMatchArray | null;
  if (match) return word(match[0], stream, state);

  if (stream.match(/^(->|==|!=|<=|>=|&&|\|\||[+\-*/%=<>!])/)) return "operator";
  if (stream.eat("(")) {
    if (state.expect === "parameters") {
      state.inParams = true;
      state.expect = "anything";
    }
    return "bracket";
  }
  if (stream.eat(")")) {
    state.inParams = false;
    return "bracket";
  }
  if (stream.match(/^[{}]/)) return "bracket";
  if (stream.match(/^[,;:]/)) return "punctuation";
  stream.next();
  return "invalid";
}

function word(w: string, stream: StringStream, state: HighlightState): string {
  if (w === "fn") {
    state.expect = "function name";
    state.params = []; // a new function: its own parameters
    return "definitionKeyword";
  }
  if (w === "let") {
    state.expect = "variable name";
    return "definitionKeyword";
  }
  if (DECLARATIONS.has(w)) return "definitionKeyword"; // `mut` keeps the expectation
  if (CONTROL.has(w)) return "controlKeyword";
  if (LITERALS.has(w)) return "bool";
  if (TYPES.has(w)) return "typeName";

  switch (state.expect) {
    case "function name":
      state.expect = "parameters";
      return "functionDef";
    case "variable name":
      state.expect = "anything";
      return "variableDef";
  }
  if (state.inParams && stream.match(/^\s*:/, false)) {
    state.params.push(w);
    return "param";
  }
  if (stream.match(/^\s*\(/, false)) return BUILTINS.has(w) ? "builtin" : "function";
  if (state.params.includes(w)) return "param";
  return "variableName";
}

// Tags for the kinds the standard set doesn't name.
const paramTag = Tag.define();

export const stepwiseLanguage = StreamLanguage.define<HighlightState>({
  name: "stepwise",
  startState,
  copyState,
  token,
  languageData: { commentTokens: { line: "//" } },
  tokenTable: {
    functionDef: t.function(t.definition(t.variableName)),
    function: t.function(t.variableName),
    builtin: t.standard(t.variableName),
    variableDef: t.definition(t.variableName),
    param: paramTag,
  },
});

// Colours come from CSS variables, so the dark and light themes both work (style.css).
const style = HighlightStyle.define([
  { tag: t.definitionKeyword, color: "var(--syn-declare)", fontWeight: "600" },
  { tag: t.controlKeyword, color: "var(--syn-control)", fontWeight: "600", fontStyle: "italic" },
  { tag: t.function(t.definition(t.variableName)), color: "var(--syn-function-def)", fontWeight: "700" },
  { tag: t.function(t.variableName), color: "var(--syn-function)" },
  { tag: t.standard(t.variableName), color: "var(--syn-builtin)" },
  { tag: paramTag, color: "var(--syn-param)", fontStyle: "italic" },
  { tag: t.definition(t.variableName), color: "var(--syn-variable-def)", fontWeight: "600" },
  { tag: t.variableName, color: "var(--syn-variable)" },
  { tag: t.typeName, color: "var(--syn-type)" },
  { tag: t.number, color: "var(--syn-number)" },
  { tag: t.bool, color: "var(--syn-bool)" },
  { tag: t.operator, color: "var(--syn-operator)" },
  { tag: [t.bracket, t.punctuation], color: "var(--syn-punctuation)" },
  { tag: t.comment, color: "var(--syn-comment)", fontStyle: "italic" },
  { tag: t.invalid, color: "var(--error)", textDecoration: "underline wavy" },
]);

export const stepwise = [stepwiseLanguage, syntaxHighlighting(style)];
