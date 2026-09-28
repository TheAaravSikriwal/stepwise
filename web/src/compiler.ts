// Types for the JSON produced by `compiler-wasm` (see compiler-wasm/src/lib.rs).
// All offsets are UTF-16 offsets into the newline-normalized source, which is
// exactly what CodeMirror positions are.

export interface Range {
  from: number;
  to: number;
}

export interface Diagnostic {
  severity: "error" | "warning";
  message: string;
  span: Range;
  labels: { span: Range; message: string }[];
  notes: string[];
  /** Terminal-style rendering with the code snippet and underline. */
  rendered: string;
}

export type TypeName = "int" | "bool";

export interface DebugTable {
  /** `returns`: what the function gives back, or null for nothing (absent in hand-made tables). */
  functions: { name: string; span: Range; returns?: TypeName | null }[];
  vars: { name: string; ty: TypeName; fnId: number; scopeId: number; span: Range }[];
  scopes: { fnId: number; parent: number | null; span: Range }[];
  /** Indexed by span_id from `line(span_id)` events. */
  steps: Range[];
}

export interface CompileMeta {
  diagnostics: Diagnostic[];
  debug: DebugTable;
}

export interface Compiled extends CompileMeta {
  /** `undefined` when there are errors. */
  wasm: Uint8Array | undefined;
}

/** The subset of the wasm-bindgen module we use, so tests can inject it. */
export interface CompilerModule {
  compile(source: string): { readonly wasm: Uint8Array | undefined; readonly meta: string };
}

export function compileWith(mod: CompilerModule, source: string): Compiled {
  const result = mod.compile(source);
  const meta = JSON.parse(result.meta) as CompileMeta;
  return { ...meta, wasm: result.wasm };
}
