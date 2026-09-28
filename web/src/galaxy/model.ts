// The call galaxy's model: which calls a run made, how they nest, when each
// one was running, and where each sits in 3D. No three.js here, so it's
// plain data that tests can check.
//
// The tree is built once from the recording. Moving through the run only
// changes which calls are visible (started), running (started and not yet
// returned), and current (the innermost running one), never the layout, so
// bubbles don't jump around as you scrub.

import type { DebugTable } from "../compiler";
import { Kind, type TraceData } from "../runtime/trace";

export interface CallNode {
  /** The call's `callId`: the index of its `call` event. */
  callId: number;
  fnId: number;
  name: string;
  /** Index into `nodes`, or -1 for the first call (`main`). */
  parent: number;
  depth: number;
  children: number[];
  /** Event index of the `call`. */
  start: number;
  /** Event index of the `ret`, or the trace's length if it never returned (the run stopped). */
  end: number;
  /** Steps that ran in this call itself (not in calls it made): its size. */
  ownSteps: number;
  /** The first step inside the call, for jumping to it. */
  firstStep: number;
  /** Calls under this one that were folded in to keep the galaxy readable. */
  hidden: number;
}

export interface CallTree {
  nodes: CallNode[];
  /** Calls left out of `nodes` (folded into an ancestor's `hidden`). */
  hiddenTotal: number;
}

/** More bubbles than this and the galaxy stops being readable (and fast). */
export const DEFAULT_CAP = 2000;

export function buildCallTree(trace: TraceData, debug: DebugTable, cap = DEFAULT_CAP): CallTree {
  // Pass 1: every call, its parent, and its own step count.
  interface Raw {
    callId: number;
    fnId: number;
    parent: number;
    end: number;
    ownSteps: number;
    firstStep: number;
    children: number[];
  }
  const all: Raw[] = [];
  const open: number[] = [];
  let step = -1;
  for (let i = 0; i < trace.length; i++) {
    switch (trace.kind[i]) {
      case Kind.Call: {
        const parent = open.length ? open[open.length - 1] : -1;
        all.push({ callId: i, fnId: trace.a[i], parent, end: trace.length, ownSteps: 0, firstStep: -1, children: [] });
        const index = all.length - 1;
        if (parent >= 0) all[parent].children.push(index);
        open.push(index);
        break;
      }
      case Kind.Line: {
        step++;
        const top = open[open.length - 1];
        if (top !== undefined) {
          all[top].ownSteps++;
          if (all[top].firstStep < 0) all[top].firstStep = step;
        }
        break;
      }
      case Kind.Ret: {
        const done = open.pop();
        if (done !== undefined) all[done].end = i;
        break;
      }
    }
  }

  // Pass 2: keep the first `cap` calls breadth-first (the top of the tree
  // matters most); fold the rest into their nearest kept ancestor.
  const subtree = new Int32Array(all.length).fill(1);
  for (let i = all.length - 1; i >= 0; i--) {
    const p = all[i].parent;
    if (p >= 0) subtree[p] += subtree[i];
  }
  const keptIndex = new Int32Array(all.length).fill(-1);
  const nodes: CallNode[] = [];
  let hiddenTotal = 0;
  const queue: number[] = all.length ? [0] : [];
  for (let q = 0; q < queue.length; q++) {
    const raw = all[queue[q]];
    const parent = raw.parent >= 0 ? keptIndex[raw.parent] : -1;
    const node: CallNode = {
      callId: raw.callId,
      fnId: raw.fnId,
      name: debug.functions[raw.fnId]?.name ?? `function ${raw.fnId}`,
      parent,
      depth: parent >= 0 ? nodes[parent].depth + 1 : 0,
      children: [],
      start: raw.callId,
      end: raw.end,
      ownSteps: raw.ownSteps,
      firstStep: Math.max(raw.firstStep, 0),
      hidden: 0,
    };
    keptIndex[queue[q]] = nodes.length;
    if (parent >= 0) nodes[parent].children.push(nodes.length);
    nodes.push(node);
    for (const child of raw.children) {
      if (nodes.length + (queue.length - q - 1) < cap) {
        queue.push(child);
      } else {
        node.hidden += subtree[child];
        hiddenTotal += subtree[child];
      }
    }
  }
  return { nodes, hiddenTotal };
}

export type Phase = "waiting" | "running" | "done";

/** Each call's phase once `eventsThrough` events have happened, and the current call's index. */
export function phasesAt(tree: CallTree, eventsThrough: number): { phases: Phase[]; current: number } {
  let current = -1;
  const phases = tree.nodes.map((n, i): Phase => {
    if (n.start >= eventsThrough) return "waiting";
    if (n.end < eventsThrough) return "done";
    if (current < 0 || n.depth > tree.nodes[current].depth) current = i;
    return "running";
  });
  return { phases, current };
}

/** How many events step k has applied (the replay engine's definition). */
export function eventsThroughStep(trace: TraceData, step: number): number {
  return step < trace.lines.length ? trace.lines[step] + 1 : trace.length;
}

// ------------------------------------------------------------------ layout

export type Vec3 = [number, number, number];

const GOLDEN = Math.PI * (3 - Math.sqrt(5));

function normalize([x, y, z]: Vec3): Vec3 {
  const len = Math.hypot(x, y, z) || 1;
  return [x / len, y / len, z / len];
}

function cross(a: Vec3, b: Vec3): Vec3 {
  return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
}

/** Two unit vectors at right angles to `d` (and each other). */
function basis(d: Vec3): [Vec3, Vec3] {
  const helper: Vec3 = Math.abs(d[1]) < 0.9 ? [0, 1, 0] : [1, 0, 0];
  const u = normalize(cross(d, helper));
  return [u, cross(d, u)];
}

/** A bubble's radius, from how long the call ran. */
export function radiusOf(node: CallNode): number {
  return Math.min(1.1, 0.3 + 0.14 * Math.cbrt(node.ownSteps + node.hidden));
}

/**
 * Where each call sits: `main` in the middle; its calls spread around it on
 * a sphere; every other call's calls fan out in a cone pointing away from
 * its parent. A single child (recursion) is turned a little each level, so
 * a recursion becomes a curling chain rather than a straight stick. The
 * layout depends only on the tree, so it never changes while you scrub.
 */
export function layout(tree: CallTree): Vec3[] {
  const pos: Vec3[] = tree.nodes.map(() => [0, 0, 0]);
  const dir: Vec3[] = tree.nodes.map(() => [0, 1, 0]);
  for (let i = 0; i < tree.nodes.length; i++) {
    const node = tree.nodes[i];
    const k = node.children.length;
    if (k === 0) continue;
    const reach = 1.4 + 2.6 * Math.pow(0.86, node.depth);
    const [u, v] = basis(dir[i]);
    node.children.forEach((child, j) => {
      let d: Vec3;
      if (node.parent < 0) {
        // Around `main`: a Fibonacci sphere, so the first calls spread evenly.
        const y = k === 1 ? 0.35 : 1 - (2 * (j + 0.5)) / k;
        const r = Math.sqrt(Math.max(0, 1 - y * y));
        const a = j * GOLDEN;
        d = [Math.cos(a) * r, y, Math.sin(a) * r];
      } else {
        // A cone around this call's own direction; one child curls.
        const spread = k === 1 ? 0.45 : Math.min(1.25, 0.5 + 0.12 * k);
        const tilt = spread * Math.sqrt((j + 0.5) / k);
        const turn = j * GOLDEN + node.depth * 1.1;
        const [dx, dy, dz] = dir[i];
        const s = Math.sin(tilt);
        const c = Math.cos(tilt);
        d = normalize([
          dx * c + (u[0] * Math.cos(turn) + v[0] * Math.sin(turn)) * s,
          dy * c + (u[1] * Math.cos(turn) + v[1] * Math.sin(turn)) * s,
          dz * c + (u[2] * Math.cos(turn) + v[2] * Math.sin(turn)) * s,
        ]);
      }
      dir[child] = d;
      pos[child] = [pos[i][0] + d[0] * reach, pos[i][1] + d[1] * reach, pos[i][2] + d[2] * reach];
    });
  }
  return pos;
}
