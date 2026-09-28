// The code editor: CodeMirror 6 with Stepwise highlighting, compiler error
// underlines, the debugger's current-step highlight, and Ctrl/Cmd+Enter to run.

import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { bracketMatching, indentOnInput, indentUnit } from "@codemirror/language";
import { type Diagnostic as LintDiagnostic, lintGutter, setDiagnostics } from "@codemirror/lint";
import { EditorState, RangeSet, StateEffect, StateField } from "@codemirror/state";
import {
  Decoration,
  type DecorationSet,
  EditorView,
  GutterMarker,
  drawSelection,
  gutter,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
} from "@codemirror/view";
import type { Diagnostic, Range } from "../compiler";
import { stepwise } from "./language";

export interface Editor {
  view: EditorView;
  source(): string;
  /** Replaces the whole program (undoable with Ctrl+Z). */
  setSource(text: string): void;
  /** Shows compiler diagnostics as underlines, or clears them with `[]`. */
  showDiagnostics(diags: Diagnostic[]): void;
  /** Highlights the debugger's current step (and scrolls to it), or clears it with `null`. */
  showStep(range: Range | null): void;
  /** Marks the line a value came from (and scrolls to it), or clears it with `null`. */
  showOrigin(range: Range | null): void;
  /** 1-based line number of an offset. */
  lineOf(offset: number): number;
  /** Called after every edit. */
  onChange(listener: () => void): void;
  /** 1-based line numbers that have a breakpoint. */
  breakpoints(): Set<number>;
}

/**
 * A highlight of one range and its line, set by an effect and cleared with
 * `null`. Two of them: the step about to run (yellow), and the line a value
 * came from (blue).
 */
function rangeHighlight(lineClass: string, rangeClass: string) {
  const set = StateEffect.define<Range | null>();
  const field = StateField.define<DecorationSet>({
    create: () => Decoration.none,
    update(deco, tr) {
      deco = deco.map(tr.changes);
      for (const effect of tr.effects) {
        if (!effect.is(set)) continue;
        if (!effect.value) {
          deco = Decoration.none;
          continue;
        }
        const len = tr.state.doc.length;
        const from = Math.min(effect.value.from, len);
        const to = Math.min(Math.max(effect.value.to, from), len);
        const line = tr.state.doc.lineAt(from);
        const ranges = [Decoration.line({ class: lineClass }).range(line.from)];
        if (to > from) ranges.push(Decoration.mark({ class: rangeClass }).range(from, to));
        deco = Decoration.set(ranges, true);
      }
      return deco;
    },
    provide: (f) => EditorView.decorations.from(f),
  });
  return { set, field };
}

const stepHighlight = rangeHighlight("cm-stepLine", "cm-stepRange");
const originHighlight = rangeHighlight("cm-originLine", "cm-originRange");

// Breakpoints: click the gutter next to a line number (or press F9) to toggle.
const toggleBreakpointAt = StateEffect.define<number>({ map: (pos, mapping) => mapping.mapPos(pos) });

const breakpointMarker = new (class extends GutterMarker {
  toDOM() {
    const dot = document.createElement("span");
    dot.className = "cm-breakpoint";
    dot.textContent = "●";
    dot.title = "Breakpoint";
    return dot;
  }
})();

const breakpointState = StateField.define<RangeSet<GutterMarker>>({
  create: () => RangeSet.empty,
  update(set, tr) {
    set = set.map(tr.changes);
    for (const effect of tr.effects) {
      if (!effect.is(toggleBreakpointAt)) continue;
      const pos = effect.value;
      let has = false;
      set.between(pos, pos, () => void (has = true));
      set = has
        ? set.update({ filter: (from) => from !== pos })
        : set.update({ add: [breakpointMarker.range(pos)] });
    }
    return set;
  },
});

function toggleBreakpoint(view: EditorView, pos: number): boolean {
  view.dispatch({ effects: toggleBreakpointAt.of(view.state.doc.lineAt(pos).from) });
  return true;
}

const breakpoints = [
  breakpointState,
  gutter({
    class: "cm-breakpoint-gutter",
    markers: (v) => v.state.field(breakpointState),
    initialSpacer: () => breakpointMarker,
    domEventHandlers: { mousedown: (view, line) => toggleBreakpoint(view, line.from) },
  }),
  keymap.of([{ key: "F9", run: (view) => toggleBreakpoint(view, view.state.selection.main.head) }]),
];

export function createEditor(parent: HTMLElement, doc: string, onRun: () => void): Editor {
  const listeners: (() => void)[] = [];
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
        breakpoints,
        lintGutter(),
        stepHighlight.field,
        originHighlight.field,
        keymap.of([runKey, ...defaultKeymap, ...historyKeymap, indentWithTab]),
        stepwise,
        EditorView.contentAttributes.of({ "aria-label": "Program source", spellcheck: "false" }),
        EditorView.updateListener.of((u) => {
          if (u.docChanged) for (const l of listeners) l();
        }),
      ],
    }),
  });

  return {
    view,
    source: () => view.state.doc.toString(),
    setSource(text) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text } });
    },
    showDiagnostics(diags) {
      view.dispatch(setDiagnostics(view.state, toLint(diags, view.state.doc.length)));
    },
    showStep(range) {
      const effects: StateEffect<unknown>[] = [stepHighlight.set.of(range)];
      if (range) {
        const pos = Math.min(range.from, view.state.doc.length);
        effects.push(EditorView.scrollIntoView(pos, { y: "nearest" }));
      }
      view.dispatch({ effects });
    },
    showOrigin(range) {
      const effects: StateEffect<unknown>[] = [originHighlight.set.of(range)];
      if (range) {
        const pos = Math.min(range.from, view.state.doc.length);
        effects.push(EditorView.scrollIntoView(pos, { y: "nearest" }));
      }
      view.dispatch({ effects });
    },
    lineOf: (offset) => view.state.doc.lineAt(Math.min(offset, view.state.doc.length)).number,
    onChange: (listener) => void listeners.push(listener),
    breakpoints() {
      const lines = new Set<number>();
      view.state.field(breakpointState).between(0, view.state.doc.length, (from) => {
        lines.add(view.state.doc.lineAt(from).number);
      });
      return lines;
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
