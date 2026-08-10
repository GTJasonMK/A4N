import { readFileSync } from "node:fs";

const baseUrl = process.env.A4N_WEB_E2E_BASE_URL ?? "http://127.0.0.1:5174";
const apiBaseUrl = process.env.A4N_WEB_E2E_API_BASE_URL ?? baseUrl;
const statePath = process.env.A4N_WEB_E2E_STATE_PATH;
const loginUsername = process.env.A4N_WEB_E2E_USERNAME;
const loginPassword = process.env.A4N_WEB_E2E_PASSWORD;

export const config = {
  runner: "local",
  specs: ["./real-postprocess.e2e.mjs"],
  maxInstances: 1,
  capabilities: [
    {
      browserName: "chrome",
      "wdio:chromedriverOptions": {
        binary: process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver",
      },
      "goog:chromeOptions": {
        binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium",
        args: [
          "--headless=new",
          "--no-sandbox",
          "--disable-dev-shm-usage",
          "--window-size=1440,1000",
        ],
      },
    },
  ],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 120_000 },
  waitforTimeout: 30_000,
  logLevel: "warn",
  baseUrl,
  before: async () => {
    await browser.url("/");
    let entries = statePath
      ? JSON.parse(readFileSync(statePath, "utf8")).entries
      : null;
    if (loginUsername && loginPassword) {
      const response = await fetch(`${apiBaseUrl}/api/auth/login`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          username: loginUsername,
          password: loginPassword,
        }),
      });
      if (!response.ok) {
        throw new Error(`web E2E 登录失败：HTTP ${response.status}`);
      }
      const result = await response.json();
      entries = {
        "a4n:current_user_id": String(result.user.id),
        "a4n:current_session_id": result.tokens.session_id,
        "a4n:current_access_token": result.tokens.access_token,
        "a4n:current_refresh_token": result.tokens.refresh_token,
      };
    }
    if (!entries) return;
    await browser.execute((entries) => {
      for (const [key, value] of Object.entries(entries)) {
        localStorage.setItem(key, value);
      }
    }, entries);
    await browser.refresh();
  },
};
