import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

const baseUrl = process.env.A4N_BROWSER_E2E_BASE_URL ?? "http://127.0.0.1:5173";
const artifactDir = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR ?? "target/browser-full-project-e2e";
const chromiumBinary = process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const chromedriverBinary = process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver";
const downloadDir = process.env.A4N_BROWSER_E2E_DOWNLOAD_DIR ?? `${artifactDir}/downloads`;

if (!process.env.CHROMEDRIVER_PATH) process.env.CHROMEDRIVER_PATH = chromedriverBinary;

mkdirSync(artifactDir, { recursive: true });
mkdirSync(downloadDir, { recursive: true });

const safeName = (value) => String(value ?? "browser-test")
  .replace(/[^a-zA-Z0-9._-]+/g, "-")
  .replace(/^-+|-+$/g, "")
  .slice(0, 120) || "browser-test";

export const config = {
  runner: "local",
  specs: ["./full-project.e2e.mjs"],
  maxInstances: 1,
  capabilities: [{
    browserName: "chrome",
    "wdio:chromedriverOptions": {},
    "goog:chromeOptions": {
      binary: chromiumBinary,
      args: [
        "--headless=new",
        "--no-sandbox",
        "--disable-dev-shm-usage",
        "--disable-gpu",
        "--lang=zh-CN",
        "--window-size=1440,1000",
      ],
      prefs: {
        "download.default_directory": downloadDir,
        "download.prompt_for_download": false,
        "download.directory_upgrade": true,
        "safebrowsing.enabled": true,
      },
    },
  }],
  framework: "mocha",
  reporters: ["spec"],
  mochaOpts: { timeout: 1_800_000 },
  waitforTimeout: 30_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 2,
  logLevel: "warn",
  baseUrl,
  before: async () => {
    await browser.sendCommand("Browser.setDownloadBehavior", {
      behavior: "allow",
      downloadPath: resolve(downloadDir),
    });
    await browser.setWindowSize(1440, 1000);
  },
  afterTest: async (test, _context, result) => {
    if (result.passed) return;
    const name = safeName(test?.title);
    try {
      await browser.saveScreenshot(`${artifactDir}/${name}.png`);
      const dom = await browser.execute(() => document.documentElement.outerHTML);
      writeFileSync(`${artifactDir}/${name}.html`, dom, "utf8");
    } catch (error) {
      process.stderr.write(`browser failure artifact capture failed: ${String(error)}\n`);
    }
  },
};
