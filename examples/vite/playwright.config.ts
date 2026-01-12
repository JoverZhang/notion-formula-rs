import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  use: {
    baseURL: "http://localhost:5173",
    browserName: "chromium",
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "pnpm run dev -- --host localhost --port 5173 --strictPort",
    url: "http://localhost:5173/?debug=1",
    reuseExistingServer: true,
    timeout: 5_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
