import { defineConfig } from "vitest/config";

export default defineConfig({
  // Relative asset paths so the site works from any subpath (e.g. GitHub Pages).
  base: "./",
  worker: { format: "es" },
  build: {
    // The call galaxy's chunk is mostly three.js (~570 kB). It only loads
    // when the galaxy tab opens, so it doesn't slow the playground down.
    chunkSizeWarningLimit: 650,
    rollupOptions: {
      input: {
        main: "index.html",
        language: "language.html",
        howItWorks: "how-it-works.html",
      },
    },
  },
  // The example gallery and doc pages are loaded from ../docs.
  server: { fs: { allow: [".."] } },
  test: {
    include: ["tests/**/*.test.ts"],
  },
});
