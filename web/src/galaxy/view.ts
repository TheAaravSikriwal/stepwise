// The call galaxy: the run as a 3D map. Every function call is a bubble,
// the calls it made cluster around it, and the variables of each running
// call orbit it as small moons. Bigger means more steps ran there. The call
// running now glows gold. Drag to look around, scroll to zoom, click a
// bubble to jump to that moment.
//
// Loaded only when the galaxy is opened (it pulls in three.js). The layout
// comes from model.ts and never changes while scrubbing: `update` only
// shows, hides and recolours.

import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import { CSS2DObject, CSS2DRenderer } from "three/examples/jsm/renderers/CSS2DRenderer.js";
import { currentTheme } from "../embed";
import { type FrameView, varKey } from "../replay/types";
import { type CallTree, type Phase, type Vec3, layout, phasesAt, radiusOf } from "./model";

/**
 * One colour per function, cycled; `main` always gets the first. Metals and
 * pearl: ivory, silver, champagne, bronze, pewter, copper. Gold itself is
 * kept for the call running now, so nothing else is mistaken for it.
 */
const PALETTES = {
  dark: ["#f4f1ea", "#c9c3b6", "#e8d5a3", "#b8925e", "#8f8a80", "#d9cfb8", "#a8744e", "#efe6d2"],
  light: ["#1a1917", "#5f5a52", "#8b6a3a", "#6d5a2e", "#3a3631", "#9a8f7a", "#7a5230", "#2d2a26"],
};
const NOW = { dark: "#f1c84b", light: "#c9a227" };
const MAX_MOONS = 400;
/** Finished calls shrink back so the calls still running stand out. */
const DONE_SCALE = 0.7;

export class GalaxyView {
  private renderer: THREE.WebGLRenderer;
  private labels: CSS2DRenderer;
  private scene = new THREE.Scene();
  private camera: THREE.PerspectiveCamera;
  private controls: OrbitControls;
  private bubbles: THREE.InstancedMesh;
  private links: THREE.LineSegments;
  private glow: THREE.Mesh;
  private moons: THREE.InstancedMesh;
  private positions: Vec3[];
  private baseColours: THREE.Color[];
  private phases: Phase[] = [];
  private current = -1;
  /** The running calls' variables, for the moons: node index → values. */
  private orbits: { node: number; vars: { label: string; changed: boolean }[] }[] = [];
  private labelObjects: CSS2DObject[] = [];
  private raycaster = new THREE.Raycaster();
  private pointer = new THREE.Vector2(-9, -9);
  private hovered = -1;
  private frame = 0;
  private resizeObserver: ResizeObserver;
  private disposed = false;
  private tooltip: HTMLElement;
  private probe: HTMLElement;
  private theme = "";
  private last: Parameters<GalaxyView["update"]> | null = null;
  private muted = new THREE.Color();
  private moonColour = new THREE.Color();
  private now = new THREE.Color(NOW.dark);
  /** The open part of the page to centre the map in, in CSS pixels; null for the whole canvas. */
  private focus: { x: number; y: number; width: number; height: number } | null = null;

  constructor(
    private readonly container: HTMLElement,
    private readonly tree: CallTree,
    private readonly onPick: (step: number) => void,
  ) {
    // Custom properties made with light-dark() read back unresolved, so
    // colours are read off an element that uses them.
    this.probe = document.createElement("span");
    this.probe.hidden = true;
    container.append(this.probe);

    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    this.renderer.setClearColor(0x000000, 0);
    container.append(this.renderer.domElement);

    this.labels = new CSS2DRenderer();
    this.labels.domElement.className = "galaxy-labels";
    container.append(this.labels.domElement);

    this.tooltip = document.createElement("div");
    this.tooltip.className = "galaxy-tooltip";
    this.tooltip.hidden = true;
    container.append(this.tooltip);

    this.camera = new THREE.PerspectiveCamera(50, 1, 0.1, 1000);
    this.controls = new OrbitControls(this.camera, this.labels.domElement);
    this.controls.enableDamping = true;
    this.controls.autoRotate = true;
    this.controls.autoRotateSpeed = 0.6;
    // Once you take hold of it, it stays where you put it.
    this.controls.addEventListener("start", () => (this.controls.autoRotate = false));

    this.scene.add(new THREE.AmbientLight(0xffffff, 1.1));
    const key = new THREE.DirectionalLight(0xffffff, 1.6);
    key.position.set(5, 8, 6);
    this.scene.add(key);

    // Bubbles: one instance per call, all in one draw.
    this.positions = layout(tree);
    this.baseColours = tree.nodes.map(() => new THREE.Color());
    this.bubbles = new THREE.InstancedMesh(
      new THREE.SphereGeometry(1, 32, 20),
      new THREE.MeshStandardMaterial({ roughness: 0.35, metalness: 0.15, transparent: true, opacity: 0.93 }),
      Math.max(tree.nodes.length, 1),
    );
    this.scene.add(this.bubbles);

    // Links from each call to the call that made it; colour alpha hides a link until its call starts.
    const pairs = tree.nodes.filter((n) => n.parent >= 0);
    const linkPositions = new Float32Array(pairs.length * 6);
    let k = 0;
    tree.nodes.forEach((n, i) => {
      if (n.parent < 0) return;
      linkPositions.set([...this.positions[n.parent], ...this.positions[i]], k);
      k += 6;
    });
    const linkGeometry = new THREE.BufferGeometry();
    linkGeometry.setAttribute("position", new THREE.BufferAttribute(linkPositions, 3));
    linkGeometry.setAttribute("color", new THREE.BufferAttribute(new Float32Array(pairs.length * 8), 4));
    this.links = new THREE.LineSegments(linkGeometry, new THREE.LineBasicMaterial({ vertexColors: true, transparent: true }));
    this.scene.add(this.links);

    // The glow around the call running now.
    this.glow = new THREE.Mesh(
      new THREE.SphereGeometry(1, 32, 20),
      new THREE.MeshBasicMaterial({ color: NOW.dark, transparent: true, opacity: 0.2, depthWrite: false }),
    );
    this.scene.add(this.glow);
    this.readTheme();

    // Moons: the running calls' variables.
    this.moons = new THREE.InstancedMesh(
      new THREE.SphereGeometry(1, 16, 12),
      new THREE.MeshStandardMaterial({ roughness: 0.4, metalness: 0.1 }),
      MAX_MOONS,
    );
    this.moons.count = 0;
    this.scene.add(this.moons);

    this.frameCamera();
    this.resizeObserver = new ResizeObserver(() => this.resize());
    this.resizeObserver.observe(container);
    this.resize();
    this.listen();
    this.animate();
  }

  /** Picks up the page's colours, once per theme change. */
  private readTheme(): void {
    const theme = currentTheme();
    if (theme === this.theme) return;
    this.theme = theme;
    const read = (name: string) => {
      this.probe.style.color = `var(${name})`;
      return getComputedStyle(this.probe).color;
    };
    this.muted.set(read("--muted"));
    this.moonColour.set(read("--text"));
    this.now.set(NOW[theme]);
    (this.glow.material as THREE.MeshBasicMaterial).color.copy(this.now);
    const palette = PALETTES[theme];
    this.tree.nodes.forEach((n, i) =>
      this.baseColours[i].set(i === 0 ? palette[0] : palette[(n.fnId % (palette.length - 1)) + 1]),
    );
  }

  /** Shows the run as of `eventsThrough` events, with the running calls' variables as moons. */
  update(eventsThrough: number, frames: FrameView[], changed: Set<string>): void {
    this.last = [eventsThrough, frames, changed];
    this.readTheme();
    const { phases, current } = phasesAt(this.tree, eventsThrough);
    this.phases = phases;
    this.current = current;

    const byCall = new Map(this.tree.nodes.map((n, i) => [n.callId, i]));
    this.orbits = frames.flatMap((f, frameIndex) => {
      const node = byCall.get(f.callId);
      if (node === undefined) return [];
      return [
        {
          node,
          vars: f.vars.map((v) => ({ label: `${v.name} = ${v.display}`, changed: changed.has(varKey(frameIndex, v.varId)) })),
        },
      ];
    });

    // Bubbles: hidden until their call starts; bright while running; faded once done.
    const matrix = new THREE.Matrix4();
    const colour = new THREE.Color();
    this.tree.nodes.forEach((node, i) => {
      const r = phases[i] === "waiting" ? 0 : phases[i] === "done" ? radiusOf(node) * DONE_SCALE : radiusOf(node);
      matrix.makeScale(r, r, r).setPosition(...this.positions[i]);
      this.bubbles.setMatrixAt(i, matrix);
      if (i === current) colour.copy(this.now);
      else if (phases[i] === "running") colour.copy(this.baseColours[i]);
      else colour.copy(this.baseColours[i]).lerp(this.muted, 0.65);
      this.bubbles.setColorAt(i, colour);
    });
    this.bubbles.instanceMatrix.needsUpdate = true;
    if (this.bubbles.instanceColor) this.bubbles.instanceColor.needsUpdate = true;

    const colours = this.links.geometry.getAttribute("color") as THREE.BufferAttribute;
    let k = 0;
    this.tree.nodes.forEach((node, i) => {
      if (node.parent < 0) return;
      const c = phases[i] === "running" ? this.baseColours[i] : this.muted;
      const alpha = phases[i] === "waiting" ? 0 : phases[i] === "running" ? 0.9 : 0.35;
      for (let end = 0; end < 2; end++) colours.setXYZW(k * 2 + end, c.r, c.g, c.b, alpha);
      k++;
    });
    colours.needsUpdate = true;

    this.renderLabels();
  }

  /**
   * Names over the running calls: `main`, the current call, and where each
   * function was first entered. The rest of a recursion (fib inside fib
   * inside fib) goes unlabelled, so the names don't pile up.
   */
  private renderLabels(): void {
    for (const o of this.labelObjects) o.removeFromParent();
    this.labelObjects = [];
    const add = (text: string, at: Vec3, className: string) => {
      const div = document.createElement("div");
      div.className = className;
      div.textContent = text;
      const label = new CSS2DObject(div);
      label.center.set(0.5, 1); // sits on top of the bubble, not over it
      label.position.set(...at);
      this.scene.add(label);
      this.labelObjects.push(label);
    };
    this.tree.nodes.forEach((node, i) => {
      if (this.phases[i] !== "running") return;
      const parent = this.tree.nodes[node.parent];
      if (i !== this.current && parent && parent.fnId === node.fnId) return;
      const [x, y, z] = this.positions[i];
      add(node.name, [x, y + radiusOf(node) + 0.08, z], i === this.current ? "galaxy-label now" : "galaxy-label");
    });
  }

  private frameCamera(): void {
    // Aim at the middle of everything, far enough back to see it all.
    const box = new THREE.Box3();
    for (const p of this.positions) box.expandByPoint(new THREE.Vector3(...p));
    const centre = box.getCenter(new THREE.Vector3());
    let radius = 1.5;
    for (const p of this.positions) radius = Math.max(radius, centre.distanceTo(new THREE.Vector3(...p)) + 1);
    this.camera.far = radius * 12;
    this.camera.updateProjectionMatrix();
    const d = radius * 2.4;
    this.camera.position.copy(centre).add(new THREE.Vector3(0.56, 0.35, 0.75).multiplyScalar(d));
    this.controls.target.copy(centre);
    this.controls.update();
  }

  /** Centres the map in part of the canvas (the rest is under the panels). */
  setFocus(rect: { x: number; y: number; width: number; height: number } | null): void {
    this.focus = rect;
    this.resize();
  }

  private resize(): void {
    const { clientWidth: w, clientHeight: h } = this.container;
    if (!w || !h) return;
    this.renderer.setSize(w, h);
    this.labels.setSize(w, h);
    this.camera.aspect = w / h;
    // Shift the picture so the camera's centre lands in the middle of the
    // open space. Picking and labels use the same projection, so they follow.
    if (this.focus) {
      const cx = this.focus.x + this.focus.width / 2;
      const cy = this.focus.y + this.focus.height / 2;
      this.camera.setViewOffset(w, h, w / 2 - cx, h / 2 - cy, w, h);
    } else {
      this.camera.clearViewOffset();
    }
    this.camera.updateProjectionMatrix();
  }

  private listen(): void {
    const el = this.labels.domElement;
    let down: { x: number; y: number } | null = null;
    el.addEventListener("pointermove", (e) => {
      const rect = el.getBoundingClientRect();
      this.pointer.set(((e.clientX - rect.left) / rect.width) * 2 - 1, -((e.clientY - rect.top) / rect.height) * 2 + 1);
      this.tooltip.style.left = `${e.clientX - rect.left + 14}px`;
      this.tooltip.style.top = `${e.clientY - rect.top + 14}px`;
    });
    el.addEventListener("pointerleave", () => this.pointer.set(-9, -9));
    el.addEventListener("pointerdown", (e) => (down = { x: e.clientX, y: e.clientY }));
    el.addEventListener("pointerup", (e) => {
      // A click, not the end of a drag: jump to that call.
      if (down && Math.hypot(e.clientX - down.x, e.clientY - down.y) < 5 && this.hovered >= 0) {
        this.onPick(this.tree.nodes[this.hovered].firstStep);
      }
      down = null;
    });
  }

  private pick(): void {
    this.raycaster.setFromCamera(this.pointer, this.camera);
    const hit = this.raycaster.intersectObject(this.bubbles)[0];
    const i = hit?.instanceId ?? -1;
    const visible = i >= 0 && this.phases[i] !== "waiting";
    this.hovered = visible ? i : -1;
    this.labels.domElement.style.cursor = visible ? "pointer" : "grab";
    if (!visible) {
      this.tooltip.hidden = true;
      return;
    }
    const n = this.tree.nodes[i];
    const phase = i === this.current ? "running now" : this.phases[i] === "running" ? "waiting for a call it made" : "finished";
    const folded = n.hidden ? ` · ${n.hidden} more calls folded in` : "";
    this.tooltip.textContent = `${n.name} · ${phase} · ${n.ownSteps} ${n.ownSteps === 1 ? "step" : "steps"}${folded} · click to jump here`;
    this.tooltip.hidden = false;
  }

  private animate = (): void => {
    if (this.disposed) return;
    requestAnimationFrame(this.animate);
    this.frame++;
    const t = performance.now() / 1000;

    // The glow breathes around the current call.
    if (this.current >= 0) {
      const r = radiusOf(this.tree.nodes[this.current]) * (1.55 + 0.12 * Math.sin(t * 3));
      this.glow.visible = true;
      this.glow.position.set(...this.positions[this.current]);
      this.glow.scale.setScalar(r);
    } else {
      this.glow.visible = false;
    }

    // Moons orbit their call; just-changed variables are gold.
    const matrix = new THREE.Matrix4();
    let m = 0;
    for (const { node, vars } of this.orbits) {
      const [x, y, z] = this.positions[node];
      const orbit = radiusOf(this.tree.nodes[node]) + 0.32;
      vars.forEach((v, j) => {
        if (m >= MAX_MOONS) return;
        const a = t * 0.9 + (j / vars.length) * Math.PI * 2 + node;
        const size = v.changed ? 0.13 : 0.09;
        matrix.makeScale(size, size, size).setPosition(x + Math.cos(a) * orbit, y + Math.sin(a * 0.7) * 0.15, z + Math.sin(a) * orbit);
        this.moons.setMatrixAt(m, matrix);
        this.moons.setColorAt(m, v.changed ? this.now : this.moonColour);
        m++;
      });
    }
    this.moons.count = m;
    this.moons.instanceMatrix.needsUpdate = true;
    if (this.moons.instanceColor) this.moons.instanceColor.needsUpdate = true;

    if (this.frame % 3 === 0) this.pick();
    // The theme button was pressed: recolour.
    if (this.frame % 30 === 0 && this.last && currentTheme() !== this.theme) this.update(...this.last);
    this.controls.update();
    this.renderer.render(this.scene, this.camera);
    this.labels.render(this.scene, this.camera);
  };

  dispose(): void {
    this.disposed = true;
    this.resizeObserver.disconnect();
    this.controls.dispose();
    this.renderer.dispose();
    this.renderer.forceContextLoss();
    this.scene.traverse((o) => {
      const mesh = o as THREE.Mesh;
      mesh.geometry?.dispose();
      const material = mesh.material as THREE.Material | THREE.Material[] | undefined;
      if (Array.isArray(material)) material.forEach((mat) => mat.dispose());
      else material?.dispose();
    });
    this.container.replaceChildren();
  }
}
