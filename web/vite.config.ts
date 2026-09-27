import { defineConfig } from "vitest/config";

export default defineConfig({
  // Relative asset paths so the site works from any subpath (e.g. GitHub Pages).
  base: "./",
  worker: { format: "es" },
  test: {
    include: ["tests/**/*.test.ts"],
  },
});
