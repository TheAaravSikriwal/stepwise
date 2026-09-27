// The code editor: CodeMirror 6 with Stepwise highlighting, compiler error
// underlines, and Ctrl/Cmd+Enter to run.

import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput, indentUnit } from "@codemirror/language";
import { type Diagnostic as LintDiagnostic, lintGutter, setDiagnostics } from "@codemirror/lint";
import { EditorState } from "@codemirror/state";
import {
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import type { Diagnostic } from "../compiler";
import { stepwise } from "./language";

export interface Editor {
  view: EditorView;
  source(): string;
  /** Shows compiler diagnostics as underlines, or clears them with `[]`. */
  showDiagnostics(diags: Diagnostic[]): void;
}

export function createEditor(parent: HTMLElement, doc: string, onRun: () => void): Editor {
  const runKey = { key: "Mod-Enter", run: () => (onRun(), true), preventDefault: true };
  const view = new EditorView({
    parent,
    state: EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        drawSelection(),
        history(),
        indentOnInput(),
        bracketMatching(),
        indentUnit.of("    "),
        EditorState.tabSize.of(4),
        lintGutter(),
        keymap.of([runKey, ...defaultKeymap, ...historyKeymap, indentWithTab]),
        stepwise,
        EditorView.contentAttributes.of({ "aria-label": "Program source", spellcheck: "false" }),
      ],
    }),
  });

  return {
    view,
    source: () => view.state.doc.toString(),
    showDiagnostics(diags) {
      view.dispatch(setDiagnostics(view.state, toLint(diags, view.state.doc.length)));
    },
  };
}

/** Converts compiler diagnostics to CodeMirror's format. Exported for tests. */
export function toLint(diags: Diagnostic[], docLength: number): LintDiagnostic[] {
  const clamp = (n: number) => Math.max(0, Math.min(n, docLength));
  return diags.map((d) => {
    const extra = [...d.labels.map((l) => l.message), ...d.notes.map((n) => `note: ${n}`)];
    return {
      from: clamp(d.span.from),
      to: clamp(d.span.to),
      severity: d.severity,
      message: [d.message, ...extra].join("\n"),
    };
  });
}
