// Copies the built playground (web/dist) into the wearechintu.com site, which
// serves it at /stepwise-app/ and frames it at /stepwise. The site is how
// Stepwise is released: test locally, push the site branch for a Vercel
// preview, merge to main to publish.
//
//   npm run export-site -- D:\VisualStudioProjects\gitbuddywebsite-stepwise
//
// It builds the compiler and the playground and runs every test (Rust and
// web) first, so an export is always a tested build. Set STEPWISE_SKIP_TESTS=1
// to skip them, for trying the plumbing before the MVP works, never for a
// release.

import { execSync } from "node:child_process";
import { cpSync, existsSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const web = resolve(fileURLToPath(new URL("..", import.meta.url)));
const site = process.argv[2];

if (!site) {
  console.error("usage: npm run export-site -- <path to the wearechintu site checkout>");
  process.exit(2);
}
const pkg = join(site, "package.json");
if (!existsSync(pkg) || JSON.parse(readFileSync(pkg, "utf8")).name !== "wearechintu") {
  console.error(`${site} doesn't look like the wearechintu site (no package.json named "wearechintu")`);
  process.exit(2);
}
const run = (cmd) => execSync(cmd, { cwd: web, stdio: "inherit" });
run("npm run wasm");
if (process.env.STEPWISE_SKIP_TESTS === "1") {
  console.warn("\nSkipping the tests (STEPWISE_SKIP_TESTS=1). Don't release this build.\n");
} else {
  run("cargo test --workspace");
  run("npm test");
}
run("npm run build");
const dist = join(web, "dist");

const target = join(site, "public", "stepwise-app");
rmSync(target, { recursive: true, force: true });
cpSync(dist, target, { recursive: true });

// Which Stepwise commit this is, so the site repo's history says what shipped.
const git = (cmd) => execSync(`git ${cmd}`, { cwd: web, encoding: "utf8" }).trim();
const commit = git("rev-parse --short HEAD");
const dirty = git("status --porcelain") !== "";
writeFileSync(
  join(target, "VERSION"),
  `stepwise ${commit}${dirty ? " (with uncommitted changes)" : ""}\n` +
    `exported ${new Date().toISOString()}\n` +
    "Built files: don't edit here. Change the Stepwise repo and export again.\n",
);

console.log(`Copied web/dist to ${target} (stepwise ${commit}${dirty ? ", uncommitted changes" : ""})`);
if (dirty) console.warn("warning: the Stepwise repo has uncommitted changes, so this build isn't reproducible");
