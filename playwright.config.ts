import { defineConfig, devices } from "@playwright/test";

// Die Tests laufen gegen den Entwicklungsserver von Vite. Die Rust-Seite ist
// dabei durch den Prüfstand in `src/lib/e2e.ts` ersetzt.
export default defineConfig({
  testDir: "./tests-ui",
  timeout: 30_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : [["list"]],
  use: {
    baseURL: "http://localhost:1420",
    trace: "retain-on-failure",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: {
    command: "npm run dev -- --port 1420 --strictPort",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
