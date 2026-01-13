/// <reference types="vitest" />
import { defineConfig } from "vite";

export default defineConfig({
  // Use repo name as base for GitHub Project Pages
  base: process.env.GITHUB_PAGES === "true" ? "/notion-formula-rs/" : "/",
  test: {
    include: ["tests/unit/**/*"],
  },
});
