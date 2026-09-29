import { config as webConfig } from "../full-project/wdio.conf.mjs";
import { writeFileSync } from "node:fs";

const application = process.env.A4N_WDIO_APP_BINARY;
if (!application) throw new Error("A4N_WDIO_APP_BINARY is required");

// Connect to tauri-driver directly; no test plugin is needed in the app.
export const config = {
  ...webConfig,
  specs: ["./specialized-agent.e2e.mjs"],
  hostname: "127.0.0.1",
  port: Number(process.env.A4N_TAURI_DRIVER_PORT || "4444"),
  capabilities: [{
    browserName: "wry",
    "wdio:enforceWebDriverClassic": true,
    "tauri:options": { application },
  }],
  before: async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.setTimeout({ script: 30_000 });
  },
  afterTest: async (_test, _context, result) => {
    if (result.passed) return;
    const directory = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
    await browser.saveScreenshot(`${directory}/native-failure.png`);
    writeFileSync(`${directory}/native-failure.html`, await browser.getPageSource());
  },
};
