import { defineConfig } from '@playwright/test';

const isCI = !!process.env.CI;

export default defineConfig({
  testDir: '.',
  timeout: 60000,
  retries: isCI ? 2 : 0,
  use: {
    baseURL: 'http://localhost:8080',
    trace: 'on-first-retry',
    navigationTimeout: isCI ? 60000 : 30000,
  },
  webServer: {
    command: 'cd .. && dx serve --example web_demo --port 8080 --platform web',
    port: 8080,
    timeout: isCI ? 300000 : 120000, // 5 min on CI, 2 min locally
    reuseExistingServer: !isCI,
  },
});
