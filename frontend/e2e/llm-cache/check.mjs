import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { remote } from "webdriverio";

const root = fileURLToPath(new URL("../..", import.meta.url));
const output = fileURLToPath(new URL("../../../data/llm-cache-browser/", import.meta.url));
const port = process.env.VITE_PORT ?? "4198";
const url = `http://127.0.0.1:${port}/e2e/llm-cache/`;
const vite = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", port, "--strictPort"], {
  cwd: root,
  env: { ...process.env, VITE_A4N_API_TRANSPORT: "tauri", VITE_A4N_API_BASE_URL: "http://127.0.0.1:19998/api", A4N_WEB_ADDR: "127.0.0.1:19998", A4N_DISABLE_VITE_HMR: "1" },
  stdio: ["ignore", "pipe", "pipe"],
});
let logs = "";
for (const stream of [vite.stdout, vite.stderr]) stream.on("data", (chunk) => { logs = `${logs}${chunk}`.slice(-8000); });
let browser;
try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (vite.exitCode != null) throw new Error(logs);
    try { ready = (await fetch(url)).ok; } catch { /* Await Vite startup. */ }
    if (ready) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.ok(ready, logs);
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
  if (browser) await browser.deleteSession();
  vite.kill("SIGTERM");
}
