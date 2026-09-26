import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { remote } from "webdriverio";
import { createServer } from "vite";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const artifacts = path.resolve(frontend, "../data/shot-sync-e2e/browser");
const fixture = JSON.parse(await readFile(process.env.A4N_SHOT_SYNC_FIXTURE ?? path.join(artifacts, "../browser-fixture.json"), "utf8"));
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, state: "running" }));
const port = 48946;
const driverPort = 48947;
const processes = [];
const logs = [];
const measurements = [];
let browser;
let vite;
const launch = (command, args) => {
  const child = spawn(command, args, { cwd: frontend, env: { ...process.env, TMPDIR: path.join(artifacts, "../tmp") }, stdio: ["ignore", "pipe", "pipe"] });
  child.stdout.on("data", (data) => logs.push(String(data)));
  child.stderr.on("data", (data) => logs.push(String(data)));
  processes.push(child);
  return child;
};
async function ready(child, url) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    assert.equal(child.exitCode, null, logs.join(""));
    try { if ((await fetch(url)).ok) return; } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Readiness timeout: ${url}`);
}
async function measure(width, state) {
  await browser.setViewport({ width, height: 900 });
  await browser.$("main").waitForDisplayed();
  await browser.execute(() => document.querySelector("main").scrollTo(0, 0));
  await browser.executeAsync((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  const result = await browser.execute(() => ({ width: innerWidth, overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth, contentOverflow: document.querySelector("main").scrollWidth - document.querySelector("main").clientWidth }));
  assert.equal(result.width, width);
  assert.equal(result.overflow, 0, `${state} overflow at ${width}px`);
  assert.equal(result.contentOverflow, 0, `${state} content overflow at ${width}px`);
  measurements.push({ state, ...result });
  await browser.saveScreenshot(path.join(artifacts, `${state}-${width}.png`));
}
try {
  vite = await createServer({ root: frontend, cacheDir: path.join(artifacts, "vite-cache"), server: { host: "127.0.0.1", port, strictPort: true, hmr: false, watch: null } });
  await vite.listen();
  const driver = launch("/usr/bin/chromedriver", [`--port=${driverPort}`]);
  await ready(driver, `http://127.0.0.1:${driverPort}/status`);
  browser = await remote({ hostname: "127.0.0.1", port: driverPort, logLevel: "error", capabilities: {
    browserName: "chrome", "goog:chromeOptions": { binary: "/usr/bin/chromium", args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(artifacts, "chrome")}`] },
  } });
  await browser.url(`http://127.0.0.1:${port}/e2e/shot-sync/bootstrap.html`);
  await browser.execute((value) => sessionStorage.setItem("shot-sync-fixture", value), JSON.stringify(fixture));
  await browser.url(`http://127.0.0.1:${port}/e2e/shot-sync/`);
  await browser.$('[aria-label="待更新镜头"]').waitForDisplayed();
  for (const width of [1440, 1024, 390]) await measure(width, "pending");
  const edit = await browser.$('[aria-label="手动修改镜头 shot-3"]');
  await edit.execute((element) => element.focus());
  await browser.keys("Enter");
  assert.equal(await browser.execute(() => document.activeElement?.closest("[data-shot-id]")?.getAttribute("data-shot-id")), "shot-3", "manual action opens and focuses its shot");
  const composition = fixture.manual.after.script.scenes[2].shots[0].composition;
  await browser.$('[data-shot-id="shot-3"]').$('label*=构图').$('textarea').setValue(composition);
  await browser.$('label*=已手动更新').$('input').click();
  for (const width of [1024, 390]) await measure(width, "editing");
  await browser.$('button=保存改编').click();
  await browser.$('[aria-label="待更新镜头"]').waitForExist({ reverse: true });
  const manual = await browser.execute(() => window.shotSyncCalls.find((call) => call.command === "edit_chapter_shooting_script"));
  assert.deepEqual(manual.input.rebind_shot_ids, ["shot-3"]);
  assert.equal(manual.input.script.scenes[2].shots[0].composition, composition);
  assert.deepEqual(manual.input.script.scenes.slice(0, 2), fixture.manual.before.script.scenes.slice(0, 2));

  await browser.url(`http://127.0.0.1:${port}/e2e/shot-sync/?mode=ai`);
  await browser.$('[aria-label="AI 更新镜头 shot-3"]').waitForDisplayed();
  await browser.$('[aria-label="AI 更新镜头 shot-3"]').click();
  await browser.$('[aria-label="待更新镜头"]').waitForExist({ reverse: true });
  const model = await browser.execute(() => window.shotSyncCalls.find((call) => call.command === "generate_chapter_shooting_script"));
  assert.deepEqual(model.input.shot_ids, ["shot-3"]);
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForExist({ reverse: true });
  await browser.$("section details").waitForDisplayed();
  await browser.waitUntil(async () => (await browser.$("main").getText()).includes(`第 ${fixture.ai.after.record.revision} 版`));
  await measure(390, "updated");
  assert.equal(await browser.$('[aria-label="待更新镜头"]').isExisting(), false);
  assert.deepEqual(await browser.execute(() => window.shotSyncErrors), []);
  await browser.url(`http://127.0.0.1:${port}/e2e/shot-sync/?mode=readonly`);
  await browser.$('[aria-label="待更新镜头"]').waitForDisplayed();
  assert.equal(await browser.$('[aria-label="AI 更新镜头 shot-3"]').isExisting(), false);
  assert.equal(await browser.$('[aria-label="手动修改镜头 shot-3"]').isExisting(), false);
  for (const mode of ["partial", "partial-failure"]) {
    await browser.url(`http://127.0.0.1:${port}/e2e/shot-sync/?mode=${mode}`);
    await browser.$('button=进入编辑').waitForDisplayed();
    await browser.$('button=进入编辑').click();
    await browser.$('details[class~="group/scene"] > summary').click();
    await browser.$('[data-shot-id="shot-1"] > summary').click();
    const composition = fixture.partial.after.script.scenes[0].shots[0].composition;
    await browser.$('[data-shot-id="shot-1"]').$('label*=构图').$('textarea').setValue(composition);
    await browser.$('button=生成全部剩余场景').click();
    await browser.waitUntil(async () => (await browser.execute(() => window.shotSyncCalls.some((call) => call.command === "edit_chapter_shooting_script_progress"))));
    if (mode === "partial") {
      await browser.waitUntil(async () => (await browser.$("main").getText()).includes(`第 ${fixture.partial.after.record.revision} 版`));
      await browser.keys("Escape");
      await browser.$('[role="dialog"]').waitForExist({ reverse: true });
      const calls = await browser.execute(() => window.shotSyncCalls);
      const savedIndex = calls.findIndex((call) => call.command === "edit_chapter_shooting_script_progress");
      const generatedIndex = calls.findIndex((call) => call.command === "generate_chapter_shooting_script");
      assert.ok(savedIndex < generatedIndex, "continuation saves the edited snapshot before generating");
      assert.equal(calls[savedIndex].input.script.scenes[0].shots[0].composition, composition);
      await measure(390, "continued");
    } else {
      await browser.waitUntil(async () => (await browser.$("body").getText()).includes("进度保存冲突"));
      assert.equal(await browser.execute(() => window.shotSyncCalls.some((call) => call.command === "generate_chapter_shooting_script")), false);
      assert.equal(await browser.$('[data-shot-id="shot-1"]').$('label*=构图').$('textarea').getValue(), composition);
    }
    assert.deepEqual(await browser.execute(() => window.shotSyncErrors), []);
  }
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: true, measurements, manualShotIds: manual.input.rebind_shot_ids, modelShotIds: model.input.shot_ids }, null, 2));
  console.log(JSON.stringify({ success: true, measurements }));
} catch (error) {
  const page = browser ? await browser.execute(() => ({ url: location.href, text: document.body.innerText, html: document.getElementById("root")?.innerHTML, errors: window.shotSyncErrors })) : null;
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, error: String(error), measurements, page }, null, 2));
  if (browser) await browser.saveScreenshot(path.join(artifacts, "failed.png"));
  throw error;
} finally {
  if (browser) await browser.deleteSession();
  if (vite) await vite.close();
  for (const child of processes.reverse()) child.kill("SIGTERM");
  await writeFile(path.join(artifacts, "process.log"), logs.join(""));
}
