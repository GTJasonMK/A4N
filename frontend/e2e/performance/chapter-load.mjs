import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { build, preview } from "vite";
import { remote } from "webdriverio";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const root = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(root, "data/performance-audit-20260927/b08/browser"));
const port = Number(process.env.A4N_PERF_PORT ?? 49030);
const baseline = process.env.A4N_PERF_BASELINE_REF;
const continueText = baseline ? "继续" : "放弃草稿";
const stayText = baseline ? "留在当前" : "取消";
const rounds = Number(process.env.A4N_PERF_ROUNDS ?? 3);
const digest = (body) => createHash("sha256").update(body).digest("hex");
await mkdir(artifacts, { recursive: true });
const temp = await mkdtemp(path.join(root, ".dev-runtime", "cl-"));
const files = execFileSync("rg", ["--files", "prompts", "-g", "*.md"], { cwd: root, encoding: "utf8" }).trim().split("\n").sort();
const corpus = (await Promise.all(files.map((file) => readFile(path.join(root, file), "utf8")))).join("\n");
assert.ok(corpus.length > 32768);
const heavyFields = ["content", "edit_revision", "analysis_data", "style_tags"];
function chapter(number) {
  const start = (number * 3571) % (corpus.length - 4000);
  return {
    id: number, project_id: 1, chapter_number: number, title: `章节 ${number} 🙂`,
    content: corpus.slice(start, start + 4000), word_count: 2, status: "draft", outline_id: null,
    edit_revision: `revision-${number}`, summary: null, style_tags: null, analysis_data: null,
    analysis_sync_status: null, analysis_sync_can_resume: false, analysis_sync_has_saved_analysis: false,
    analysis_sync_resume_block_reason: null, analysis_sync_error: null,
    derived_sync_status: "completed", derived_sync_workflow_run_id: null, derived_sync_can_resume: false,
    derived_sync_error: null, material_updates: [], created_at: null, updated_at: null,
    material_sync: { content_index: "completed", outline_and_foreshadowing: "not_analyzed",
      objects: "not_analyzed", characters: "not_analyzed", scenes: "not_analyzed" },
  };
}
let chapters = Array.from({ length: 1000 }, (_, i) => chapter(i + 1));
let requests = [];
const readDelays = new Map();
const readFailures = new Set();
let saveFailure = false;
let saveDelay = 0;
const unexpected = [];
const apiError = (message) => ({ code: "internal", message, retryable: false, trace_id: null, details: {} });
const report = { success: false, baseline: baseline ?? null, samples: [], checks: [] };
const server = createServer(async (req, res) => {
  try {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    const body = chunks.length ? JSON.parse(Buffer.concat(chunks)) : {};
    const url = new URL(req.url, "http://localhost");
    const command = url.pathname.split("/").at(-1);
    requests.push({ command, body });
    let result;
    let status = 200;
    switch (command) {
      case "get_project": result = { id: 1, title: "性能基准", project_type: "novel_long", project_kind: "text",
        my_role: "owner", lifecycle_gate: "active", source_sync_status: "current", chapter_count: chapters.length }; break;
      case "list_chapters": result = chapters; break;
      case "list_chapter_index": result = chapters.map((chapter) => ({
        ...Object.fromEntries(Object.entries(chapter).filter(([key]) => !heavyFields.includes(key))),
        body_character_count: Array.from(chapter.content).length,
      })); break;
      case "get_chapter": {
        assert.equal(body.input.project_id, 1);
        const selected = chapters.find((chapter) => chapter.id === body.input.chapter_id);
        result = selected ? { ...selected } : null;
        await delay(readDelays.get(body.input.chapter_id) ?? 0);
        if (!result || readFailures.has(body.input.chapter_id)) { status = 500; result = apiError("fixture chapter read failed"); }
        break;
      }
      case "update_chapter_content": {
        await delay(saveDelay);
        const selected = chapters.find((chapter) => chapter.id === body.input.chapter_id);
        assert.equal(body.input.expected_revision, selected.edit_revision);
        if (saveFailure) { status = 500; result = apiError("fixture save failed"); break; }
        Object.assign(selected, { content: body.input.content, edit_revision: selected.edit_revision + "-saved" });
        result = selected; break;
      }
      case "list_chapter_outlines": case "list_profiles": case "active": result = []; break;
      case "get_blueprint": result = null; break;
      default: unexpected.push(url.pathname); status = 500; result = { error: `Unexpected fixture request: ${url.pathname}` };
    }
    const raw = Buffer.from(JSON.stringify(result));
    const encoded = gzipSync(raw);
    res.writeHead(status, { "Content-Type": "application/json", "Content-Encoding": "gzip",
      "Content-Length": encoded.length, "Cache-Control": "no-store" });
    res.end(encoded);
  } catch (error) {
    unexpected.push(error.stack); res.writeHead(500); res.end("Fixture assertion failed");
  }
});
let previewServer;
let driver;
let browser;
async function cdp(cmd, params = {}) {
  const response = await fetch(`http://127.0.0.1:${port + 1}/session/${browser.sessionId}/goog/cdp/execute`, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ cmd, params }),
  });
  assert.ok(response.ok); return (await response.json()).value;
}
const editor = () => browser.$(".a4n-writing-workspace textarea");
async function waitBody(expected) {
  await browser.waitUntil(async () => await editor().isExisting() && await editor().getValue() === expected,
    { timeout: 30000, timeoutMsg: "Editor did not load the selected persisted body" });
}
async function button(text) {
  const found = await browser.$(`button*=${text}`); await found.waitForDisplayed(); return found;
}
async function clickChapter(number) {
  const row = await browser.$(`.a4n-chapter-card*=${`第 ${number} 章`}`);
  if (await row.isExisting()) await row.click();
  else {
    const rows = await browser.$$(".a4n-chapter-card");
    assert.ok(rows.length >= number, `Missing chapter ${number}`); await rows[number - 1].click();
  }
}
try {
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const sourceFiles = ["src/pages/WritingDesk.tsx", "src/pages/novel-detail/ChaptersTab.tsx", "src/pages/novel-detail/OptimizeTab.tsx",
    "src/pages/novel-detail/optimize/OptimizeHeader.tsx", ...execFileSync("rg", ["--files", "src/pages/writing-desk"], { cwd: frontend, encoding: "utf8" }).trim().split("\n")];
  const override = new Map(baseline ? sourceFiles.filter((file) => !file.endsWith("/chapterIndex.ts")).map((file) => [path.join(frontend, file),
    execFileSync("git", ["show", `${baseline}:frontend/${file}`], { cwd: root, encoding: "utf8" })]) : []);
  // Keep unrelated concurrent WIP out of this reproducible build without touching its files.
  const unrelated = ["src/components/feedback/UnsavedChangesGuard.tsx"];
  for (const file of unrelated) override.set(path.join(frontend, file), execFileSync("git", ["show", `HEAD:frontend/${file}`], { cwd: root, encoding: "utf8" }));
  const outDir = path.resolve(process.env.A4N_PERF_DIST ?? path.join(artifacts, "dist"));
  if (!process.env.A4N_PERF_DIST) await build({ root: frontend, logLevel: "warn", plugins: [{ name: "chapter-baseline", enforce: "pre", load: (id) => override.get(id) }],
    build: { outDir, emptyOutDir: true, rollupOptions: { input: path.join(frontend, "e2e/performance/chapter-load.html") } } });
  previewServer = await preview({ root: frontend, build: { outDir }, preview: { host: "127.0.0.1", port, strictPort: true,
    proxy: { "/api": `http://127.0.0.1:${server.address().port}` } } });
  driver = spawn("/usr/bin/chromedriver", [`--port=${port + 1}`], { cwd: frontend, env: { ...process.env, TMPDIR: temp }, stdio: "ignore" });
  for (let tries = 0; ; tries++) {
    assert.equal(driver.exitCode, null); assert.ok(tries < 100, "Driver startup timeout");
    try { if ((await fetch(`http://127.0.0.1:${port + 1}/status`)).ok) break; } catch {}
    await delay(100);
  }
  browser = await remote({ hostname: "127.0.0.1", port: port + 1, logLevel: "error", capabilities: { browserName: "chrome",
    "goog:chromeOptions": { binary: "/usr/bin/chromium", args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(temp, "p")}`] } } });
  await browser.setWindowSize(1440, 900);
  await browser.url(`http://127.0.0.1:${port}/e2e/performance/chapter-load.html`);
  await browser.waitUntil(() => browser.execute(() => Boolean(window.chapterLoad)), { timeout: 30000 });
  await browser.execute(() => {
    window.fixtureErrors = [];
    window.addEventListener("error", (event) => window.fixtureErrors.push(event.error?.stack ?? event.message));
    window.addEventListener("unhandledrejection", (event) => window.fixtureErrors.push(String(event.reason)));
  });
  await cdp("Network.enable");
  await cdp("Network.emulateNetworkConditions", { offline: false, latency: 50, downloadThroughput: 500000, uploadThroughput: 500000 });
  for (let round = 0; round < rounds; round++) {
    requests = [];
    const result = await browser.executeAsync((done) => {
      const started = performance.now(); window.chapterLoad.mount();
      const timer = setInterval(() => {
        if (window.fixtureErrors.length) { clearInterval(timer); done({ error: window.fixtureErrors }); return; }
        const editor = document.querySelector(".a4n-writing-workspace textarea");
        if (!editor?.value || editor.readOnly || editor.disabled) return;
        clearInterval(timer); done({ elapsedMs: performance.now() - started, body: editor.value,
          resources: performance.getEntriesByType("resource").filter((entry) => entry.startTime >= started && entry.name.includes("/api/"))
            .map((entry) => ({ name: entry.name, encoded: entry.encodedBodySize, decoded: entry.decodedBodySize })) });
      }, 10);
    });
    assert.equal(result.error, undefined, JSON.stringify(result.error));
    assert.equal(result.body, chapters[0].content);
    assert.equal(requests.filter((req) => req.command === "list_chapters").length, baseline ? 1 : 0);
    if (!baseline) assert.deepEqual(requests.filter((req) => req.command === "get_chapter").map((req) => req.body.input.chapter_id), [1]);
    delete result.body; report.samples.push(result);
  }
  report.checks.push("same first chapter body; no eager body requests");
  await cdp("Network.emulateNetworkConditions", { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
  chapters = [chapter(1), chapter(2), chapter(3)];
  await browser.execute(() => window.chapterLoad.mount());
  await waitBody(chapters[0].content);
  await editor().setValue("未保存🙂é");
  await clickChapter(2);
  await (await button(stayText)).click();
  await waitBody("未保存🙂é");
  await clickChapter(2);
  if (!baseline) await browser.execute(() => {
    window.originalStorageSetItem = Storage.prototype.setItem;
    Storage.prototype.setItem = function(key, value) {
      if (key.startsWith("a4n:writing_draft:")) throw new Error("fixture local draft write failed");
      return window.originalStorageSetItem.call(this, key, value);
    };
  });
  await (await button(continueText)).click();
  if (!baseline) {
    await browser.waitUntil(() => browser.execute(() => window.chapterLoad.toasts().some((toast) => toast.message.includes("fixture local draft write failed"))));
    await waitBody("未保存🙂é");
    assert.equal(await (await button(continueText)).isDisplayed(), true);
    await browser.execute(() => { Storage.prototype.setItem = window.originalStorageSetItem; });
    await (await button(continueText)).click();
    report.checks.push("draft write failure blocks navigation and keeps unsaved content");
  }
  await waitBody(chapters[1].content);
  assert.equal(await browser.execute(() => JSON.parse(localStorage.getItem("a4n:writing_draft:1:1")).content), "未保存🙂é");
  report.checks.push("unsaved switch cancellation and old-chapter draft identity");
  const readsBeforeContext = requests.filter((req) => req.command === "get_chapter").length;
  await (await button("写作助手")).click();
  const context = await browser.$("summary*=参考上下文"); await context.waitForDisplayed();
  assert.equal(requests.filter((req) => req.command === "get_chapter").length, readsBeforeContext);
  await context.click();
  const previousTail = chapters[0].content.trim().slice(-180);
  await browser.waitUntil(async () => (await browser.$('aside[aria-label="写作助手"]').getProperty("textContent")).includes(previousTail));
  if (!baseline) assert.equal(requests.filter((req) => req.command === "get_chapter").length, readsBeforeContext + 1);
  await (await button("写作助手")).click();
  report.checks.push("previous chapter context is loaded when expanded and retains its exact ending");
  if (!baseline) {
    readDelays.set(1, 500); readDelays.set(3, 80);
    await clickChapter(1); await clickChapter(3); await waitBody(chapters[2].content); await delay(600);
    await waitBody(chapters[2].content);
    report.checks.push("late read cannot replace a newer chapter");
    readFailures.add(2); await clickChapter(2);
    await browser.waitUntil(() => browser.execute(() => document.body.textContent.includes("fixture chapter read failed")));
    assert.equal(await editor().isExisting(), false);
    readFailures.clear(); readDelays.clear(); await clickChapter(3); await waitBody(chapters[2].content);
    report.checks.push("failed body read stays visible and cannot create an empty edit");
  }
  if (baseline) { await clickChapter(3); await waitBody(chapters[2].content); }
  await editor().setValue("保存失败保留草稿"); saveFailure = true;
  await browser.keys(["Control", "s", "NULL"]);
  await browser.waitUntil(() => browser.execute(() => window.chapterLoad.toasts().some((toast) => toast.message.includes("fixture save failed"))));
  await waitBody("保存失败保留草稿"); saveFailure = false;
  await browser.keys(["Control", "s", "NULL"]);
  await browser.waitUntil(() => chapters[2].content === "保存失败保留草稿");
  await waitBody("保存失败保留草稿");
  report.checks.push("save failure preserves editor; explicit retry commits exact content");
  saveDelay = 300;
  await editor().setValue("保存中的正文");
  await browser.keys(["Control", "s", "NULL"]);
  await editor().setValue("保存中的正文，继续修改🙂");
  await browser.waitUntil(() => chapters[2].content === "保存中的正文");
  await delay(350); await waitBody("保存中的正文，继续修改🙂"); saveDelay = 0;
  report.checks.push("edits made during save are not overwritten by its response");
  if (!baseline) readDelays.set(1, 1000);
  await clickChapter(1); await (await button(continueText)).click();
  if (!baseline) {
    await delay(250);
    assert.equal(await browser.$("button*=恢复草稿").isExisting(), false);
  }
  await (await button("恢复草稿")).click(); await waitBody("未保存🙂é");
  readDelays.clear();
  report.checks.push("draft is restored only after the selected persisted chapter is available");
  for (const width of [1440, 390]) {
    await browser.setWindowSize(width, 900);
    assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
    await browser.saveScreenshot(path.join(artifacts, `writing-${width}.png`));
  }
  await browser.setWindowSize(1440, 900);
  requests = [];
  await browser.execute(() => window.chapterLoad.mount(1, "preview"));
  await (await button("章节 2")).click();
  const article = await browser.$("article");
  await browser.waitUntil(async () => await article.isExisting() && await article.getProperty("textContent") === chapters[1].content);
  await browser.keys("Escape");
  await browser.waitUntil(async () => !await browser.$("article").isExisting());
  if (!baseline) assert.deepEqual(requests.filter((req) => req.command === "get_chapter").map((req) => req.body.input.chapter_id), [2]);
  report.checks.push("read-only chapter preview fetches the selected body and closes with Escape");
  assert.deepEqual(unexpected, []);
  report.corpusSha256 = digest(corpus);
  report.success = true;
} catch (error) {
  report.error = error.stack;
  if (browser) {
    report.bodyText = await browser.execute(() => document.body.textContent);
    report.browserErrors = await browser.execute(() => window.fixtureErrors);
    await browser.saveScreenshot(path.join(artifacts, "failure.png"));
  }
  throw error;
} finally {
  report.unexpected = unexpected;
  report.requests = requests;
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  await browser?.deleteSession();
  if (driver) await stopProcess(driver);
  await previewServer?.httpServer.close(); server.close();
  await rm(temp, { recursive: true, force: true });
}
