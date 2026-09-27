import { startVite, stopProcess } from "../process.mjs";

const host = "127.0.0.1";
const port = 4174;
const baseUrl = `http://${host}:${port}`;
const chromiumBinary = process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const chromedriverBinary = process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver";
let viteProcess;

export const config = {
  runner: "local",
  specs: ["./video-mode-switch.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{
    browserName: "chrome",
    "wdio:chromedriverOptions": { binary: chromedriverBinary },
    "goog:chromeOptions": {
      binary: chromiumBinary,
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", "--window-size=1440,1000"],
    },
  }],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 60_000 },
  waitforTimeout: 20_000,
  logLevel: "warn",
  baseUrl,
  onPrepare: async () => {
    viteProcess = await startVite(port, "/e2e/video-mode-switch/", { VITE_A4N_API_TRANSPORT: "tauri" });
  },
  onComplete: () => stopProcess(viteProcess),
};
