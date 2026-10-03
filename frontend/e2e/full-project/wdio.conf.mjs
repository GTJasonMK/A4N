import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { captureBrowserDiagnostics } from "./user-flow.mjs";

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
    "goog:loggingPrefs": { browser: "ALL", performance: "ALL" },
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
    await browser.sendCommand("Page.addScriptToEvaluateOnNewDocument", {
      source: `(() => {
        const original = console.error.bind(console);
        const printable = (value) => {
          if (value instanceof Error) return value.stack ?? value.message;
          if (!value || typeof value !== "object") return value;
          try {
            return JSON.stringify(value, (key, item) => /password|token|secret|api[_-]?key|base64|authorization/i.test(key) ? "[REDACTED]" : item);
          } catch {
            return String(value);
          }
        };
        console.error = (...args) => original(...args.map(printable), new Error("Browser error stack").stack);
      })();`,
    });
    await browser.sendCommand("Browser.setDownloadBehavior", {
      behavior: "allow",
      downloadPath: resolve(downloadDir),
    });
    await browser.setWindowSize(1440, 1000);
  },
  afterTest: async (test, _context, result) => {
    const name = safeName(test?.title);
    let diagnosticError;
    try {
      const errors = await captureBrowserDiagnostics(`${artifactDir}/${name}`);
      if (errors.length) diagnosticError = new Error(`浏览器异常或服务端错误：${JSON.stringify(errors)}`);
    } catch (error) {
      diagnosticError = error;
    }
    if (!result.passed || diagnosticError) {
      for (const capture of [
        () => browser.saveScreenshot(`${artifactDir}/${name}.png`),
        async () => writeFileSync(`${artifactDir}/${name}.html`, await browser.execute(() => document.documentElement.outerHTML), "utf8"),
      ]) {
        try { await capture(); } catch (error) {
          process.stderr.write(`browser failure artifact capture failed: ${String(error)}\n`);
        }
      }
    }
    if (diagnosticError) throw diagnosticError;
  },
};
