import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { remote } from "webdriverio";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const artifacts = path.resolve(frontend, "../data/director-ui");
const temporary = path.resolve(frontend, "../../../.tmp/director-v3-ui");
await mkdir(temporary, { recursive: true });
await mkdir(artifacts, { recursive: true });
const environment = { ...process.env, TMPDIR: temporary };
const port = 43871;
const driverPort = 43872;
const processes = [];
let browser;
const logs = [];
const launch = (command, args) => {
  const child = spawn(command, args, { cwd: frontend, env: environment, stdio: ["ignore", "pipe", "pipe"] });
  child.stdout.on("data", (chunk) => logs.push(String(chunk)));
  child.stderr.on("data", (chunk) => logs.push(String(chunk)));
  processes.push(child);
  return child;
};
async function ready(child, url) {
  const until = Date.now() + 30_000;
  while (Date.now() < until) {
    if (child.exitCode != null) throw new Error(`Child exited: ${logs.join("")}`);
    try { if ((await fetch(url)).ok) return; } catch { /* process is starting */ }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Server readiness timeout: ${url}`);
}
try {
  const vite = launch(process.execPath, ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", String(port), "--strictPort"]);
  const driver = launch(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", [`--port=${driverPort}`, "--verbose", `--log-path=${path.join(artifacts, "driver.log")}`]);
  await Promise.all([ready(vite, `http://127.0.0.1:${port}/e2e/director-timeline/`), ready(driver, `http://127.0.0.1:${driverPort}/status`)]);
  browser = await remote({ hostname: "127.0.0.1", port: driverPort, path: "/", logLevel: "error", capabilities: {
    browserName: "chrome", "goog:chromeOptions": { binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium", args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(artifacts, "chrome")}`, "--window-size=1440,1000"] },
  } });
  await browser.setWindowSize(1440, 1000);
  await browser.url(`http://127.0.0.1:${port}/e2e/director-timeline/`);
  await browser.$(".director-workspace").waitForExist({ timeout: 20_000 });
  assert.equal((await browser.$$(".director-point")).length, 7);
  assert.equal((await browser.$$(".director-segment")).length, 3);
  const frame = await browser.$(".director-viewer").getSize();
  assert.ok(Math.abs(frame.width / frame.height - 16 / 9) < 0.02);
  const setInput = (selector, value) => browser.execute((selector, value) => {
    const input = document.querySelector(selector);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, selector, value);
  await setInput('[aria-label="播放位置"]', "3000");
  await browser.waitUntil(async () => (await browser.$(".director-frame-caption").getText()).includes("2s"));
  await setInput('[aria-label="播放位置"]', "5000");
  await browser.waitUntil(async () => (await browser.$(".director-frame-caption").getText()).includes("4s"));
  await browser.$('[aria-label="播放分镜"]').click();
  await browser.waitUntil(async () => (await browser.$(".director-timecode").getText()).includes("5.") || (await browser.$(".director-frame-caption").getText()).includes("6s"));
  await browser.$('[aria-label="暂停"]').click();
  await browser.saveScreenshot(path.join(artifacts, "desktop.png"));
  await browser.$$(".director-segment")[0].click();
  const prompt = await browser.$(".director-inspector textarea");
  await prompt.setValue("镜头缓缓推近，旅人在门前停步。");
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "save_chapter_visual_plan_draft" && call.input.plan.segments[0].prompt.includes("缓缓推近"))));
  await browser.saveScreenshot(path.join(artifacts, "desktop-properties.png"));
  await browser.$('[aria-label="收起属性"]').click();
  await setInput('[aria-label="整体时长（秒）"]', "24");
  await browser.$('[aria-label="整体时长（秒）"]').click();
  await browser.keys("Tab");
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.duration_ms === 24000)));
  await browser.waitUntil(async () => (await browser.$(".director-ruler").getText()).includes("24s"));
  const measurements = [];
  for (const width of [1440, 1024, 390]) {
    await browser.setWindowSize(width, 1000);
    await browser.waitUntil(async () => browser.execute(() => document.documentElement.scrollWidth === document.documentElement.clientWidth));
    const measurement = await browser.execute(() => ({ width: innerWidth, overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth }));
    measurements.push(measurement);
    await browser.saveScreenshot(path.join(artifacts, `width-${width}.png`));
  }
  await browser.$$(".director-point")[1].click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await browser.waitUntil(async () => browser.execute(() => getComputedStyle(document.querySelector('[role="dialog"]')).opacity === "1"));
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  await browser.saveScreenshot(path.join(artifacts, "mobile-properties.png"));
  const timeBeforeDialogKeys = await browser.$(".director-timecode").getText();
  await browser.keys("Home");
  assert.equal(await browser.$(".director-timecode").getText(), timeBeforeDialogKeys);
  const splitButton = await browser.$("button=从此处分镜");
  await splitButton.scrollIntoView({ block: "center" });
  assert.ok(await splitButton.isClickable());
  const reachable = await browser.execute((button) => {
    const rect = button.getBoundingClientRect();
    return rect.top >= 0 && rect.bottom <= innerHeight;
  }, splitButton);
  assert.ok(reachable, "mobile inspector bottom actions must be fully visible after scrolling");
  await splitButton.click();
  await browser.$("h3=拆分镜头").waitForDisplayed();
  await browser.keys("Escape");
  await browser.$("h3=拆分镜头").waitForDisplayed({ reverse: true });
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.$('[aria-label="放大视图"]').click();
  assert.ok((await browser.$(".director-timeline-track").getSize()).width >= 640);
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  await browser.saveScreenshot(path.join(artifacts, "mobile-expanded.png"));
  await browser.setWindowSize(1440, 1000);
  const center = (element) => browser.execute((node) => { const r = node.getBoundingClientRect(); return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }; }, element);
  const drag = async (start, end) => {
    await browser.performActions([{ type: "pointer", id: "timeline-mouse", parameters: { pointerType: "mouse" }, actions: [
      { type: "pointerMove", duration: 0, ...start }, { type: "pointerDown", button: 0 },
      { type: "pointerMove", duration: 350, ...end }, { type: "pointerUp", button: 0 },
    ] }]);
    await browser.releaseActions();
  };
  const movingPoint = await browser.$$(".director-point")[1];
  await movingPoint.scrollIntoView({ block: "center" });
  const pointCenter = await center(movingPoint);
  await drag(pointCenter, { x: pointCenter.x + 40, y: pointCenter.y });
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.kind === "move_point" && call.input.operation.point_id === "p1" && call.input.operation.at_ms > 4000 && call.input.operation.at_ms < 8000)));
  const boundary = await browser.$$(".director-segment-boundary")[0];
  await boundary.waitForEnabled();
  const boundaryCenter = await center(boundary);
  const targetCenter = await center(await browser.$$(".director-point")[2]);
  await drag(boundaryCenter, { x: targetCenter.x, y: boundaryCenter.y });
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.kind === "move_segment_boundary" && call.input.operation.point_id === "p2")));
  assert.equal((await browser.$$(".director-point")).length, 7);
  assert.equal((await browser.$$(".director-segment")).length, 3);
  await browser.execute(() => window.holdDirectorEdit());
  const completedEdits = await browser.execute(() => window.directorEditFinished ?? 0);
  await setInput('[aria-label="整体时长（秒）"]', "36");
  await browser.$('[aria-label="整体时长（秒）"]').click();
  await browser.keys("Tab");
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.duration_ms === 36000)));
  await browser.execute(() => window.switchDirectorChapter(2));
  await browser.waitUntil(async () => (await browser.$(".director-header h2").getText()) === "第二章");
  assert.ok(await browser.$('[aria-label="保存草稿"]').isEnabled(), "a pending edit in the previous chapter must not lock this chapter");
  await browser.execute(() => window.releaseDirectorEdit());
  await browser.waitUntil(async () => browser.execute((count) => window.directorEditFinished > count, completedEdits));
  await browser.execute(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await browser.$('[aria-label="整体时长（秒）"]').getValue(), "12");
  assert.doesNotMatch(await browser.$(".director-save-status").getText(), /保存中/);
  await browser.execute(() => window.switchDirectorChapter(3));
  await browser.waitUntil(async () => (await browser.$(".director-error").getText()).includes("重新加载"));
  assert.equal(await browser.$(".director-viewer").isExisting(), false, "a failed chapter load must never display the previous chapter's plan");
  const report = { passed: true, cases: ["aspect ratio", "points independent of segments", "play/pause", "prompt autosave payload", "proportional retime request", "responsive overflow", "mobile dialog and Escape", "mobile bottom actions reachable", "dialog keyboard isolation", "point and segment boundary pointer gestures", "chapter switch during pending mutation", "chapter load failure never displays the previous plan"], measurements };
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
} catch (error) {
  if (browser) {
    await browser.saveScreenshot(path.join(artifacts, "failure.png"));
    await writeFile(path.join(artifacts, "failure.html"), await browser.getPageSource());
  }
  throw error;
} finally {
  if (browser) await browser.deleteSession();
  for (const child of processes) if (child.exitCode == null) child.kill("SIGTERM");
  await writeFile(path.join(artifacts, "process.log"), logs.join(""));
}
