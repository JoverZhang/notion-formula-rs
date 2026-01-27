import { defineConfig, devices } from "@playwright/test";

const HOST = process.env.PW_HOST ?? "127.0.0.1";
const PORT = process.env.PW_PORT ?? "5173";
const BASE_URL = `http://${HOST}:${PORT}`;

export default defineConfig({
  testDir: "./tests/e2e",
  fullyParallel: true,
  use: {
    // Default to IPv4 to avoid environments that can't bind ::1 (IPv6 localhost).
    // Override via `PW_HOST=localhost` if needed.
    baseURL: BASE_URL,
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
    command: `pnpm build && pnpm vite preview --host ${HOST} --port ${PORT} --strictPort`,
    url: `${BASE_URL}/?debug=1`,
    reuseExistingServer: !process.env.CI,
    timeout: 30_000,
    stdout: "pipe",
    stderr: "pipe",
  },
});
