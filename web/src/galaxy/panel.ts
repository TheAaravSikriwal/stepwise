// The "Call galaxy" view, beside "Your code". The map fills the whole page
// behind the panels (the page's CSS turns them to frosted glass when
// html[data-view="galaxy"]). It follows the debugger: every step moves the
// galaxy to that moment, and clicking a bubble moves the debugger there.
// three.js is only fetched the first time the view opens.

import type { ReplayState } from "../replay/types";
import type { TraceData } from "../runtime/trace";
import type { Debugger, Session } from "../ui/debugger";
import { type CallTree, buildCallTree, callDetails, eventsThroughStep } from "./model";
import type { GalaxyView } from "./view";

interface Elements {
  codeTab: HTMLButtonElement;
  galaxyTab: HTMLButtonElement;
  galaxy: HTMLElement;
  /** The panels over the map, so the map can centre in the space left over. */
  side: HTMLElement;
  explain: HTMLElement;
}

export class GalaxyPanel {
  private open = false;
  private session: Session | null = null;
  private state: ReplayState | null = null;
  private view: GalaxyView | null = null;
  /** The recording the view was built from; a new run means a new galaxy. */
  private viewTrace: TraceData | null = null;
  private tree: CallTree | null = null;
  private loading: Promise<typeof import("./view")> | null = null;
  private stage: HTMLElement;
  private legend: HTMLElement;

  constructor(
    private readonly el: Elements,
    private readonly debug: Debugger,
  ) {
    this.stage = document.createElement("div");
    this.stage.className = "galaxy-stage";
    this.legend = document.createElement("div");
    this.legend.className = "galaxy-legend";
    el.galaxy.append(this.stage, this.legend);

    el.codeTab.addEventListener("click", () => this.show(false));
    el.galaxyTab.addEventListener("click", () => this.show(true));
    addEventListener("resize", () => this.refocus());
    debug.onStep((session, state) => {
      this.session = session;
      this.state = state;
      if (this.open) void this.sync();
    });
    this.notice("Press ▶ Run, then come back here to see your program's function calls as a 3D map.");
  }

  private show(galaxy: boolean): void {
    this.open = galaxy;
    document.documentElement.dataset.view = galaxy ? "galaxy" : "code";
    this.el.galaxy.hidden = !galaxy;
    this.el.codeTab.setAttribute("aria-selected", String(!galaxy));
    this.el.galaxyTab.setAttribute("aria-selected", String(galaxy));
    if (galaxy) {
      this.refocus();
      void this.sync();
    }
  }

  /** The open part of the page: left of the panels, below the explainer. */
  private refocus(): void {
    if (!this.view || !this.open) return;
    const side = this.el.side.getBoundingClientRect();
    const top = this.el.explain.getBoundingClientRect().bottom;
    const narrow = side.left < innerWidth * 0.3; // stacked (small screens): use it all
    this.view.setFocus(
      narrow
        ? { x: 0, y: top, width: innerWidth, height: innerHeight - top }
        : { x: 0, y: top, width: side.left, height: innerHeight - top },
    );
  }

  private async sync(): Promise<void> {
    const trace = this.session?.trace;
    const state = this.state;
    if (!this.session || !state) {
      this.drop();
      this.notice("Press ▶ Run, then come back here to see your program's function calls as a 3D map.");
      return;
    }
    if (!trace) {
      this.drop();
      this.notice("The galaxy needs a real run. Open the playground and press ▶ Run.");
      return;
    }
    if (this.viewTrace !== trace) {
      this.drop();
      this.viewTrace = trace;
      this.tree = buildCallTree(trace, this.session.debug);
      this.notice("Loading the galaxy…");
      this.loading ??= import("./view");
      let mod: typeof import("./view");
      try {
        mod = await this.loading;
      } catch {
        this.loading = null;
        this.viewTrace = null;
        this.notice("The galaxy couldn't load. Check your connection and open this tab again.");
        return;
      }
      // Another run (or leaving debugging) while it loaded.
      if (this.viewTrace !== trace || !this.tree) return;
      this.stage.replaceChildren();
      try {
        const debug = this.session.debug;
        this.view = new mod.GalaxyView(this.stage, this.tree, {
          onPick: (step) => this.debug.seek(step),
          details: (node, eventsThrough) => callDetails(trace, debug, node, eventsThrough),
        });
      } catch (e) {
        console.error(e);
        this.notice("Your browser can't draw 3D here (WebGL is off or unavailable), so the galaxy can't show.");
        return;
      }
      this.describe(this.tree);
      this.refocus();
    }
    const current = this.state;
    if (this.view && current) {
      this.view.update(eventsThroughStep(trace, current.step), current.frames, current.changed);
    }
  }

  private drop(): void {
    this.view?.dispose();
    this.view = null;
    this.viewTrace = null;
    this.tree = null;
    this.stage.replaceChildren();
  }

  private notice(text: string): void {
    this.legend.replaceChildren();
    const p = document.createElement("p");
    p.className = "notice galaxy-notice";
    p.textContent = text;
    this.stage.replaceChildren(p);
  }

  private describe(tree: CallTree): void {
    const calls = tree.nodes.length + tree.hiddenTotal;
    const count = (n: number) => n.toLocaleString("en-US");
    const folded =
      tree.hiddenTotal > 0 ? ` To keep it readable, ${count(tree.hiddenTotal)} later ones are folded into their callers.` : "";
    const lines = [
      `Each bubble is a function call (${count(calls)} in this run), with the calls it made around it. Bigger ones ran more steps.${folded}`,
      "Gold is running now, faded ones have finished, and the moons are variables. Drag to look around, scroll to zoom, hover a bubble or moon for its steps and values, click to jump there.",
    ];
    this.legend.replaceChildren(
      ...lines.map((text) => {
        const p = document.createElement("p");
        p.textContent = text;
        return p;
      }),
    );
  }
}
