// The trace: every event a program emitted, in order.
//
// Stored as a struct of arrays (one Int32Array per field) instead of an array
// of objects. A run can have a million events; typed arrays keep that compact
// and can be transferred from the worker without copying.

/** Event kinds. Field meanings are listed next to each (a, b, c). */
export const Kind = {
  /** a = span_id. Marks the start of a step. */
  Line: 0,
  /** a = var_id, b = value */
  Declare: 1,
  /** a = var_id, b = old value, c = new value */
  Assign: 2,
  /** a = fn_id */
  Call: 3,
  /** a = return value (0 if none) */
  Ret: 4,
  /** a = scope_id */
  ScopeExit: 5,
  /** a = value, b = 1 if it should display as a bool */
  Print: 6,
} as const;
export type Kind = (typeof Kind)[keyof typeof Kind];

/** An Int32Array backed by a plain (transferable) ArrayBuffer. */
type I32 = Int32Array<ArrayBuffer>;

/** Plain-data form of a trace, safe to `postMessage` with its buffers transferred. */
export interface TraceData {
  length: number;
  kind: I32;
  a: I32;
  b: I32;
  c: I32;
  /** Event index of every `Line` event, in order: step N starts at `lines[N]`. */
  lines: I32;
}

export class TraceBuilder {
  length = 0;
  private lineCount = 0;
  private kind: I32 = new Int32Array(1024);
  private a: I32 = new Int32Array(1024);
  private b: I32 = new Int32Array(1024);
  private c: I32 = new Int32Array(1024);
  private lines: I32 = new Int32Array(256);

  push(kind: Kind, a = 0, b = 0, c = 0): void {
    if (this.length === this.kind.length) this.grow();
    const i = this.length++;
    this.kind[i] = kind;
    this.a[i] = a;
    this.b[i] = b;
    this.c[i] = c;
    if (kind === Kind.Line) {
      if (this.lineCount === this.lines.length) this.lines = grow(this.lines);
      this.lines[this.lineCount++] = i;
    }
  }

  private grow(): void {
    this.kind = grow(this.kind);
    this.a = grow(this.a);
    this.b = grow(this.b);
    this.c = grow(this.c);
  }

  /** Trims the arrays to size. The builder must not be used afterwards. */
  finish(): TraceData {
    const n = this.length;
    return {
      length: n,
      kind: this.kind.slice(0, n),
      a: this.a.slice(0, n),
      b: this.b.slice(0, n),
      c: this.c.slice(0, n),
      lines: this.lines.slice(0, this.lineCount),
    };
  }
}

function grow(arr: I32): I32 {
  const next = new Int32Array(arr.length * 2);
  next.set(arr);
  return next;
}

export function transferables(t: TraceData): ArrayBuffer[] {
  return [t.kind, t.a, t.b, t.c, t.lines].map((arr) => arr.buffer);
}

/** Formats a `Print` event's value the way the program printed it. */
export function printed(t: TraceData, i: number): string {
  return t.b[i] ? String(t.a[i] !== 0) : String(t.a[i]);
}

/** All output lines, in order. */
export function outputOf(t: TraceData): string[] {
  const out: string[] = [];
  for (let i = 0; i < t.length; i++) if (t.kind[i] === Kind.Print) out.push(printed(t, i));
  return out;
}
