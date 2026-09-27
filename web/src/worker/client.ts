// Page-side handle to the worker. One request at a time; a new run cancels
// the previous one. If the worker stops responding, it is replaced.

import type { Request, Response } from "./protocol";

/** Safety net only: the event limit normally stops runaway programs first. */
const WATCHDOG_MS = 10_000;

export class Runner {
  private worker = Runner.spawn();
  private nextId = 1;
  private pending: { id: number; resolve: (r: Response) => void; timer: number } | null = null;

  private static spawn(): Worker {
    return new Worker(new URL("./worker.ts", import.meta.url), { type: "module" });
  }

  run(source: string): Promise<Response> {
    this.cancel();
    const id = this.nextId++;
    return new Promise((resolve) => {
      const timer = window.setTimeout(() => {
        this.restart();
        resolve({ type: "internal-error", id, message: "The program took too long and was stopped." });
      }, WATCHDOG_MS);
      this.pending = { id, resolve, timer };
      this.worker.onmessage = ({ data }: MessageEvent<Response>) => {
        if (this.pending?.id !== data.id) return;
        window.clearTimeout(this.pending.timer);
        this.pending = null;
        resolve(data);
      };
      this.worker.postMessage({ type: "run", id, source } satisfies Request);
    });
  }

  private cancel(): void {
    if (!this.pending) return;
    window.clearTimeout(this.pending.timer);
    // Still running: the only way to stop it is to replace the worker.
    this.restart();
    this.pending.resolve({ type: "internal-error", id: this.pending.id, message: "cancelled" });
    this.pending = null;
  }

  private restart(): void {
    this.worker.terminate();
    this.worker = Runner.spawn();
  }
}
