import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

import { remote } from "webdriverio";

const root = fileURLToPath(new URL("../..", import.meta.url));
const output = fileURLToPath(new URL("../../../data/llm-presets-e2e/", import.meta.url));
const port = process.env.VITE_PORT ?? "4197";
const url = `http://127.0.0.1:${port}/e2e/llm-presets/`;
let browser;
let logs = "";
const vite = spawn(process.execPath, ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", port, "--strictPort"], {
  cwd: root,
  env: { ...process.env, VITE_A4N_API_TRANSPORT: "tauri", VITE_A4N_API_BASE_URL: "http://127.0.0.1:19997/api", A4N_WEB_ADDR: "127.0.0.1:19997", A4N_DISABLE_VITE_HMR: "1" },
  stdio: ["ignore", "pipe", "pipe"],
});
for (const stream of [vite.stdout, vite.stderr]) stream.on("data", (chunk) => { logs = `${logs}${chunk}`.slice(-8000); });

try {
  let ready = false;
  for (let attempt = 0; attempt < 100; attempt++) {
    if (vite.exitCode != null) throw new Error(logs);
    try { ready = (await fetch(url)).ok; } catch { /* Vite is starting. */ }
    if (ready) break;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  assert.ok(ready, logs);
  browser = await remote({
    logLevel: "error",
    capabilities: {
      browserName: "chrome",
      "wdio:chromedriverOptions": { binary: "/usr/bin/chromedriver" },
      "goog:chromeOptions": { binary: "/usr/bin/chromium", args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage"] },
    },
  });
  await browser.url(url);
  const preset = await browser.$('[role="combobox"][aria-label="厂商预设"]');
  await preset.waitForDisplayed({ timeout: 20000 });
  await preset.click();
  await browser.$('[role="option"]*=OpenAI · Responses').click();
  const advanced = await browser.$("textarea");
  await advanced.setValue('{"prompt_cache_options":{"mode":"explicit"}}');
  assert.ok((await browser.execute(() => window.__LLM_PRESET_E2E__.form.extra_body_text)).includes("prompt_cache_options"));
  await preset.click();
  const option = await browser.$('[role="option"]*=Google Gemini');
  await option.click();
  const form = await browser.execute(() => window.__LLM_PRESET_E2E__.form);
  assert.equal(form.provider, "gemini");
  assert.equal(form.api_base_url, "https://generativelanguage.googleapis.com/v1beta");
  assert.equal(form.model, "");
  assert.equal(form.clear_api_key, true);
  assert.equal(form.extra_body_text, "");
  assert.equal(await advanced.getValue(), "");
  const key = await browser.$('input[type="password"]');
  assert.ok(await key.isEnabled());
  await key.setValue("fixture-key");
  assert.equal(await browser.execute(() => window.__LLM_PRESET_E2E__.form.clear_api_key), false);
  const detect = await browser.$('button[title="检测模型列表"]');
  await detect.click();
  await browser.waitUntil(async () => (await browser.execute(() => window.__LLM_PRESET_E2E__.calls)).some((call) => call.command === "detect_llm_config_models"));
  await browser.waitUntil(async () => (await browser.$('body').getText()).includes("已加载 1"));
  assert.equal(await browser.execute(() => window.__LLM_PRESET_E2E__.form.model), "fixture-model");
  await preset.click();
  await browser.keys("Escape");
  assert.equal(await preset.getAttribute("aria-expanded"), "false");
  await mkdir(output, { recursive: true });
  for (const width of [1440, 1024, 390]) {
    await browser.setWindowSize(width, 1100);
    const overflow = await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
    assert.equal(overflow, 0, `horizontal overflow at ${width}px`);
    await browser.saveScreenshot(`${output}/${width}.png`);
  }
  await preset.click();
  await browser.$('[role="option"]*=Anthropic Claude').click();
  await browser.execute(() => { window.__LLM_PRESET_E2E__.emptyDiscovery = true; });
  await detect.click();
  await browser.waitUntil(async () => (await browser.$('body').getText()).includes("未发现可用模型"));
  assert.equal(await browser.execute(() => window.__LLM_PRESET_E2E__.form.model), "");
  await browser.execute(() => { window.__LLM_PRESET_E2E__.failDiscovery = true; });
  await detect.click();
  await browser.waitUntil(async () => {
    const alerts = await browser.$$('[role="alert"]');
    for (const alert of alerts) {
      if (await alert.isDisplayed() && (await alert.getText()).includes("测试：模型列表暂不可用")) return true;
    }
    return false;
  });
  console.log("LLM preset browser checks passed: switching, native option reset, credentials, model discovery, empty/error states, keyboard, 1440/1024/390px.");
} catch (error) {
  if (browser) {
    await mkdir(output, { recursive: true });
    await browser.saveScreenshot(`${output}/failure.png`);
    console.error(JSON.stringify(await browser.execute(() => ({
      calls: window.__LLM_PRESET_E2E__?.calls,
      text: document.body.innerText,
    })), null, 2));
  }
  throw error;
} finally {
  if (browser) await browser.deleteSession();
  vite.kill("SIGTERM");
}
