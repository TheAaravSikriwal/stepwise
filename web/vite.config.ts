import { defineConfig } from "vitest/config";

export default defineConfig({
  // Relative asset paths so the site works from any subpath (e.g. GitHub Pages).
  base: "./",
  worker: { format: "es" },
  // The example gallery is loaded from ../docs/examples.
  server: { fs: { allow: [".."] } },
  test: {
    include: ["tests/**/*.test.ts"],
  },
});
