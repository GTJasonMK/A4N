import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { setTimeout as delay } from "node:timers/promises";
import { build, preview } from "vite";
import { remote } from "webdriverio";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(frontend, "../data/performance-audit-20260927/frontend"));
const port = Number(process.env.A4N_PERF_PORT ?? 48966);
const outDir = path.join(artifacts, "dist");
const baselineRef = process.env.A4N_PERF_BASELINE_REF;
const scenario = process.env.A4N_PERF_CASE ?? "core";
assert.ok(["core", "sidebar", "story-events", "manga-source", "galgame-source"].includes(scenario));
const entry = scenario === "sidebar" ? "sidebar.html" : scenario === "story-events" ? "story-events.html"
  : scenario === "manga-source" ? "manga-source.html"
  : scenario === "galgame-source" ? "galgame-source.html" : "index.html";
const sourceFiles = scenario === "story-events"
  ? ["src/pages/novel-detail/story-events/storyEventUtils.ts", "src/pages/novel-detail/story-events/ChapterEventSection.tsx"]
  : scenario === "sidebar"
  ? ["src/pages/writing-desk/ChapterSidebar.tsx", "src/pages/writing-desk/ChapterCard.tsx"]
  : scenario === "manga-source"
  ? ["src/pages/novel-detail/MangaTab.tsx"]
  : scenario === "galgame-source"
  ? ["src/pages/novel-detail/GalgameTab.tsx"]
  : ["src/hooks/usePacedStreamText.ts", "src/i18n/format.ts", "src/components/ui/MarkdownText.tsx"];
const baselineSources = new Map(baselineRef ? sourceFiles.map((file) => [
  path.join(frontend, file),
  execFileSync("git", ["show", `${baselineRef}:frontend/${file}`], { cwd: frontend, encoding: "utf8" }),
]) : []);
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, state: "running" }));
const tempRoot = path.resolve(process.env.A4N_PERF_TMPDIR ?? path.join(frontend, "../.dev-runtime"));
await mkdir(tempRoot, { recursive: true });
const temp = await mkdtemp(path.join(tempRoot, "perf-"));
let driver;
let server;
let browser;
let driverLog = "";
try {
  if (process.env.A4N_PERF_REUSE_BUILD !== "1") await build({ root: frontend, logLevel: "warn", plugins: [{
    name: "performance-baseline", enforce: "pre", load: (id) => baselineSources.get(id),
  }], build: {
    outDir, emptyOutDir: true,
    rollupOptions: { input: path.join(frontend, "e2e/performance", entry) },
  } });
  server = await preview({ root: frontend, build: { outDir }, preview: { host: "127.0.0.1", port, strictPort: true } });
  driver = spawn(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", [`--port=${port + 1}`, `--log-path=${path.join(artifacts, "chromedriver.log")}`], {
    cwd: frontend, env: { ...process.env, TMPDIR: temp }, stdio: ["ignore", "pipe", "pipe"],
  });
  driver.stdout.on("data", (data) => { driverLog += data; });
  driver.stderr.on("data", (data) => { driverLog += data; });
  const deadline = Date.now() + 30_000;
  while (true) {
    assert.equal(driver.exitCode, null, driverLog);
    try {
      const response = await fetch(`http://127.0.0.1:${port + 1}/status`, { signal: AbortSignal.timeout(1000) });
      if (response.ok) break;
    } catch (error) {
      if (!["TypeError", "TimeoutError"].includes(error.name)) throw error;
    }
    assert.ok(Date.now() < deadline, `ChromeDriver readiness timeout: ${driverLog}`);
    await delay(100);
  }
  browser = await remote({ hostname: "127.0.0.1", port: port + 1, logLevel: "error", capabilities: {
    browserName: "chrome", "goog:chromeOptions": {
      binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium",
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage",
        ...(scenario === "manga-source" || scenario === "galgame-source" ? ["--js-flags=--expose-gc"] : []),
        `--user-data-dir=${path.join(temp, "chrome")}`],
    },
  } });
  await browser.setTimeout({ script: 60_000 });
  const variant = baselineRef ? "before" : "after";
  await browser.url(`http://127.0.0.1:${port}/e2e/performance/${entry}?variant=${variant}`);
  await browser.waitUntil(() => browser.execute(() => window.performanceReady === true), { timeout: 30_000 });
  let report;
  if (scenario === "sidebar") {
    const traces = [];
    for (const width of [1440, 390]) {
      await browser.setViewport({ width, height: 900 });
      const trace = [];
      for (const locale of ["zh-CN", "en-US"]) {
        trace.push(await browser.execute((locale) => window.verifySidebar(locale), locale));
        await browser.execute(() => document.querySelectorAll(".a4n-chapter-card")[0].focus());
        await browser.keys("Enter");
        assert.deepEqual(await browser.execute(() => window.sidebarSelection()), [1002]);
      }
      traces.push(trace);
      assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
      await browser.saveScreenshot(path.join(artifacts, `sidebar-${width}.png`));
    }
    assert.deepEqual(traces[0], traces[1]);
    const sidebar = [];
    for (const count of [100, 1000]) sidebar.push(await browser.execute((count) => window.measureSidebar(count, 4000), count));
    report = { success: true, scenario, baselineRef: baselineRef ?? null, browser: browser.capabilities.browserVersion,
      viewports: [1440, 390], trace: traces[0], sidebar };
  } else if (scenario === "story-events") {
    const traces = [];
    for (const width of [1440, 390]) {
      await browser.setViewport({ width, height: 900 });
      const trace = [];
      for (const locale of ["zh-CN", "en-US"]) {
        trace.push(await browser.execute((locale) => window.verifyStoryEvents(locale), locale));
        await browser.execute(() => document.querySelector("section section button").focus());
        await browser.keys("Enter");
        assert.deepEqual(await browser.execute(() => window.storyEventsOpened()), [1001]);
      }
      traces.push(trace);
      assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
      await browser.saveScreenshot(path.join(artifacts, `story-events-${width}.png`));
    }
    assert.deepEqual(traces[0], traces[1]);
    const storyEvents = [];
    for (const count of [100, 1000]) storyEvents.push(await browser.execute((count) => window.measureStoryEvents(count), count));
    report = { success: true, scenario, baselineRef: baselineRef ?? null, browser: browser.capabilities.browserVersion,
      viewports: [1440, 390], trace: traces[0], storyEvents };
  } else if (scenario === "galgame-source") {
    const galgameSource = await browser.execute(() => window.measureGalgameSource());
    assert.equal(galgameSource.chapterCount, 1000);
    const workspaceRequests = galgameSource.requests.filter((request) =>
      request.command === "get_media_authoring_workspace",
    );
    assert.equal(workspaceRequests.filter((request) => request.scope_chapter_id == null).length, baselineRef ? 1 : 0);
    assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
    if (!baselineRef) {
      await browser.execute(() => document.querySelector("[role=tab][id$='-tab-adaptation']")?.click());
      await browser.waitUntil(() => browser.execute(() => window.galgameWorkspaceCompleteCount() === 1));
    }
    report = { success: true, scenario, baselineRef: baselineRef ?? null, browser: browser.capabilities.browserVersion,
      galgameSource };
  } else if (scenario === "manga-source") {
    const mangaSource = await browser.execute(() => window.measureMangaSource());
    assert.equal(mangaSource.selectedChapterReady, true);
    assert.equal(mangaSource.chapterCount, 1000);
    assert.ok(mangaSource.documentScrollWidth <= mangaSource.documentWidth);
    const workspaceRequests = mangaSource.requests.filter((request) =>
      request.command === "get_media_authoring_workspace",
    );
    assert.equal(workspaceRequests.filter((request) => request.scope_chapter_id == null).length, baselineRef ? 1 : 0);
    assert.ok(workspaceRequests.length >= 1);
    const viewports = [];
    const keyboardSwitch = [];
    for (const [index, width] of [1440, 1024, 768, 390, 375, 320].entries()) {
      const chapterNumber = index + 2;
      await browser.setViewport({ width, height: 900 });
      const scrollWidth = await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
      assert.equal(scrollWidth, 0);
      const started = await browser.execute(() => performance.now());
      await browser.execute((chapterNumber) => document.querySelectorAll(".a4n-media-sidebar-surface button")[chapterNumber - 1].focus(), chapterNumber);
      await browser.keys("Enter");
      await browser.waitUntil(() => browser.execute((number) => window.mangaSelectedChapterLabel()?.includes(`性能章节 ${number}`) === true, chapterNumber));
      await browser.waitUntil(() => browser.execute((number) => window.mangaSelectedChapterReady(number), chapterNumber));
      if (!baselineRef) {
        await browser.waitUntil(() => browser.execute((number) => window.mangaDetailLoaded(number), chapterNumber));
      } else {
        assert.equal(await browser.execute((number) => window.mangaDetailLoaded(number), chapterNumber), false);
      }
      keyboardSwitch.push({
        width,
        chapterNumber,
        elapsedMs: await browser.execute((started) => performance.now() - started, started),
      });
      await browser.saveScreenshot(path.join(artifacts, `manga-source-${width}.png`));
      viewports.push({ width, scrollWidth });
    }
    assert.deepEqual(await browser.execute(() => window.mangaUnexpectedRequests()), []);
    report = { success: true, scenario, baselineRef: baselineRef ?? null, browser: browser.capabilities.browserVersion,
      viewports, keyboardSwitch, mangaSource };
  } else {
    const traces = [];
    for (const width of [1440, 390]) {
      await browser.setViewport({ width, height: 900 });
      traces.push(await browser.execute(() => window.verifyStream()));
      assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
    }
    assert.deepEqual(traces[0], traces[1]);
    const streams = [];
    for (const characters of [1000, 10000, 50000]) {
      streams.push(await browser.execute((count) => window.measureStream(count), characters));
    }
    const formatting = await browser.execute(() => window.measureFormatting());
    const markdown = await browser.execute(() => window.measureMarkdown());
    report = { success: true, baselineRef: baselineRef ?? null, browser: browser.capabilities.browserVersion, viewports: [1440, 390], trace: traces[0], streams, formatting, markdown };
  }
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ ...report, trace: undefined }, null, 2));
} catch (error) {
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, state: "failed", error: String(error) }, null, 2));
  throw error;
} finally {
  await browser?.deleteSession();
  await stopProcess(driver);
  if (server) await new Promise((resolve, reject) => server.httpServer.close((error) => error ? reject(error) : resolve()));
  await rm(temp, { recursive: true, force: true });
}
