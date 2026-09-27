import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { remote } from "webdriverio";
import { startVite, stopProcess } from "../process.mjs";

const output = fileURLToPath(new URL("../../../data/llm-cache-browser/", import.meta.url));
const port = process.env.VITE_PORT ?? "4198";
const url = `http://127.0.0.1:${port}/e2e/llm-cache/`;
const vite = await startVite(port, "/e2e/llm-cache/", {
  VITE_A4N_API_TRANSPORT: "tauri", VITE_A4N_API_BASE_URL: "http://127.0.0.1:19998/api",
  A4N_WEB_ADDR: "127.0.0.1:19998", A4N_DISABLE_VITE_HMR: "1",
});
let browser;
try {
  browser = await remote({ logLevel: "error", capabilities: {
    browserName: "chrome", "wdio:chromedriverOptions": { binary: "/usr/bin/chromedriver" },
    "goog:chromeOptions": { binary: "/usr/bin/chromium", args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage"] },
  } });
  await browser.url(url);
  await browser.waitUntil(async () => (await browser.$("body").getText()).includes("缓存写入令牌"));
  let text = await browser.$("body").getText();
  assert.ok(text.includes("90%") && text.includes("400"), text);
  assert.ok(!text.includes("100%"), text);
  await mkdir(output, { recursive: true });
  for (const width of [1440, 1024, 390]) {
    await browser.setWindowSize(width, 1100);
    assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0, `overflow at ${width}px`);
    await browser.saveScreenshot(`${output}/${width}.png`);
  }
  const refresh = await browser.$('button[aria-label="刷新生成调用分析"]');
  await browser.execute(() => { window.__CACHE_E2E__.unknown = true; });
  await refresh.click();
  await browser.waitUntil(async () => (await browser.execute(() => window.__CACHE_E2E__.calls)) >= 2);
  await browser.waitUntil(async () => !(await browser.$("body").getText()).includes("90%"));
  await browser.saveScreenshot(`${output}/unknown-390.png`);
  await browser.execute(() => { window.__CACHE_E2E__.failure = true; });
  await refresh.click();
  await browser.waitUntil(async () => (await browser.$("body").getText()).includes("测试：统计暂不可用"));
  await browser.saveScreenshot(`${output}/error-390.png`);
  await browser.execute(() => { window.__CACHE_E2E__.failure = false; window.__CACHE_E2E__.unknown = false; });
  await browser.execute(() => document.querySelector('button[aria-label="刷新生成调用分析"]').focus());
  await browser.keys("Enter");
  await browser.waitUntil(async () => (await browser.$("body").getText()).includes("90%"));
  console.log("Cache analytics passed: reported reads/misses/writes, unknown/error states, keyboard refresh, and 1440/1024/390px overflow checks.");
} catch (error) {
  if (browser) {
    await mkdir(output, { recursive: true });
    await browser.saveScreenshot(`${output}/failure.png`);
    console.error(await browser.$("body").getText());
  }
  throw error;
} finally {
  try {
    if (browser) await browser.deleteSession();
  } finally {
    await stopProcess(vite);
  }
}
