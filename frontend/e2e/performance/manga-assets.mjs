import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer as createTcpServer } from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { gzipSync } from "node:zlib";
import { build, preview } from "vite";
import { remote } from "webdriverio";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const root = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(root, "data/performance-audit-20261015/manga-assets"));
const baselineRef = process.env.A4N_PERF_BASELINE_REF ?? "7bb6866c2";
const chapterCount = Number(process.env.A4N_PERF_CHAPTER_COUNT ?? 1000);
const imageCounts = (process.env.A4N_PERF_IMAGE_COUNTS ?? "5000,10000").split(",").map(Number);
const rounds = Number(process.env.A4N_PERF_ROUNDS ?? 3);
assert.ok(Number.isSafeInteger(chapterCount) && chapterCount > 0);
assert.ok(imageCounts.length > 0 && imageCounts.every((count) => Number.isSafeInteger(count) && count >= 0));
assert.ok(Number.isSafeInteger(rounds) && rounds > 0);

const mangaTabPath = path.join(frontend, "src/pages/novel-detail/MangaTab.tsx");
const baselineSource = execFileSync(
  "git",
  ["show", baselineRef + ":frontend/src/pages/novel-detail/MangaTab.tsx"],
  { cwd: root, encoding: "utf8" },
);
const promptFiles = execFileSync("rg", ["--files", "prompts", "-g", "*.md"], { cwd: root, encoding: "utf8" })
  .trim().split("\n").filter(Boolean).sort();
const promptCorpus = (await Promise.all(promptFiles.map((file) => readFile(path.join(root, file), "utf8")))).join("\n").replace(/\s+/g, " ");
assert.ok(promptCorpus.length > 4096, "Need representative project prompt text");

const sampleText = (index, length) => {
  const start = (index * 1537) % (promptCorpus.length - length);
  return promptCorpus.slice(start, start + length);
};
const chapterStatuses = Array.from({ length: chapterCount }, (_, index) => ({
  can_cancel: false,
  can_resume: false,
  cancelled: 0,
  chapter_id: index + 1,
  checkpoint_valid: false,
  completed: 0,
  current: null,
  failed: 0,
  generating: 0,
  has_checkpoint: false,
  message: null,
  pending: 0,
  pipeline_status: null,
  stage: null,
  task_handle: null,
  total: null,
}));
const chapters = Array.from({ length: chapterCount }, (_, index) => ({
  analysis_sync_can_resume: false,
  analysis_sync_error: null,
  analysis_sync_has_saved_analysis: false,
  analysis_sync_resume_block_reason: null,
  analysis_sync_status: null,
  body_character_count: 4096,
  chapter_number: index + 1,
  created_at: "2026-01-01T00:00:00.000Z",
  derived_sync_can_resume: false,
  derived_sync_error: null,
  derived_sync_status: "completed",
  derived_sync_workflow_run_id: null,
  id: index + 1,
  material_sync: {
    characters: "completed",
    content_index: "completed",
    objects: "completed",
    outline_and_foreshadowing: "completed",
    scenes: "completed",
  },
  material_updates: [],
  outline_id: null,
  project_id: 1,
  status: "completed",
  summary: null,
  title: "Chapter " + (index + 1),
  updated_at: null,
  word_count: 4096,
}));
const profiles = Array.from({ length: Math.max(1, Math.floor(chapterCount / 20)) }, (_, index) => ({
  age: "成年",
  appearance: sampleText(index, 180),
  attribute_counts: { explicit: 8, implicit: 4, social: 2 },
  background: sampleText(index + 17, 240),
  base_attributes: { role: "supporting", alignment: "neutral", traits: ["patient", "observant"] },
  character_arc: { start: sampleText(index + 31, 90), end: sampleText(index + 43, 90) },
  created_at: "2026-01-01T00:00:00.000Z",
  current_state: { location: "city", condition: "active" },
  gender: "未说明",
  id: index + 1,
  initial_profile_editable: true,
  last_synced_chapter: chapterCount,
  name: "Character " + (index + 1),
  personality: sampleText(index + 53, 160),
  project_id: 1,
  timeline_started: true,
  updated_at: null,
  voice_fingerprint: { register: "mid", rhythm: sampleText(index + 67, 64) },
}));
const portraits = Array.from({ length: profiles.length * 4 }, (_, index) => ({
  created_at: "2026-01-01T00:00:00.000Z",
  expression: index % 3 === 0 ? "neutral" : "thoughtful",
  id: index + 1,
  image_path: "projects/1/portraits/" + (index + 1) + ".png",
  is_primary: index % 4 === 0,
  portrait_type: "half_body",
  pose: "standing",
  profile_id: Math.floor(index / 4) + 1,
  project_id: 1,
  prompt: sampleText(index + 79, 140),
}));
const imagesByCount = new Map(imageCounts.map((count) => [count, Array.from({ length: count }, (_, index) => ({
  ai_score: index % 10 === 0 ? 0.91 : null,
  chapter_id: (index % chapterCount) + 1,
  created_at: "2026-01-01T00:00:00.000Z",
  file_path: "projects/1/images/" + (index + 1) + ".png",
  file_size: 1320000,
  height: 1536,
  id: index + 1,
  image_type: "manga_page",
  is_favorite: index % 100 === 0,
  is_system_managed: false,
  metadata_json: {
    generation_task_id: "task-" + (index + 1),
    provider_request_id: "request-" + (index + 1),
    reference_image_ids: [index % Math.max(1, profiles.length) + 1],
    stage: "image",
  },
  model: "fixture-image-model-v1",
  panel_id: "page-" + (index % 12 + 1),
  project_id: 1,
  prompt: sampleText(index + 97, 240),
  width: 1024,
}))]));

const imageConfig = [{
  backend: "fixture",
  base_url: "https://fixture.invalid",
  has_api_key: false,
  id: 1,
  is_default: true,
  max_concurrent: 1,
  model: "fixture-image-model-v1",
  name: "Fixture",
  pricing: null,
  supported_sizes: [{ width: 1024, height: 1024 }],
}];
const emptyStatus = chapterStatuses[0];
const compressedResponses = new Map();
function prepareResponse(command, value) {
  const raw = Buffer.from(JSON.stringify(value));
  const compressed = gzipSync(raw, { level: 1 });
  compressedResponses.set(command, { raw, compressed });
}
prepareResponse("list_chapter_index", chapters);
prepareResponse("list_manga_pipeline_statuses", chapterStatuses);
prepareResponse("list_profiles", profiles);
prepareResponse("list_portraits_by_project", portraits);
prepareResponse("list_image_configs", imageConfig);
prepareResponse("list_manga_page_prompts", []);
prepareResponse("list_manga_pipeline_artifacts", []);
prepareResponse("get_manga_pipeline_status", emptyStatus);
prepareResponse("get_chapter", {
  analysis_data: null,
  chapter_number: 1,
  content: sampleText(11, 4096),
  id: 1,
  project_id: 1,
  status: "completed",
  summary: null,
  title: "Chapter 1",
});
for (const [count, images] of imagesByCount) prepareResponse("list_images_by_project:" + count, images);

let activeImageCount = imageCounts[0];
let pendingRequests = 0;
let requests = [];
let unexpected = [];
const apiServer = createServer(async (req, res) => {
  const url = new URL(req.url, "http://127.0.0.1");
  if (url.pathname === "/__stats") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ pending: pendingRequests, requests, unexpected }));
    return;
  }
  if (url.pathname === "/__configure") {
    const chunks = [];
    for await (const chunk of req) chunks.push(chunk);
    const input = JSON.parse(Buffer.concat(chunks).toString("utf8"));
    assert.ok(imagesByCount.has(input.image_count));
    activeImageCount = input.image_count;
    requests = [];
    unexpected = [];
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end("{}");
    return;
  }
  if (!url.pathname.startsWith("/api/commands/")) {
    res.writeHead(404);
    res.end();
    return;
  }
  const command = decodeURIComponent(url.pathname.slice("/api/commands/".length));
  const chunks = [];
  for await (const chunk of req) chunks.push(chunk);
  const body = chunks.length ? JSON.parse(Buffer.concat(chunks).toString("utf8")) : {};
  const responseKey = command === "list_images_by_project"
    ? "list_images_by_project:" + activeImageCount
    : command;
  const prepared = compressedResponses.get(responseKey);
  if (!prepared) unexpected.push(command);
  const output = prepared ?? {
    raw: Buffer.from(JSON.stringify({
      code: "internal",
      message: "Unexpected fixture request: " + command,
      retryable: false,
      trace_id: null,
      details: {},
    })),
  };
  const compressed = output.compressed ?? gzipSync(output.raw, { level: 1 });
  pendingRequests += 1;
  requests.push({
    command,
    request_bytes: Buffer.concat(chunks).length,
    response_bytes: output.raw.length,
    response_wire_bytes: compressed.length,
  });
  res.writeHead(prepared ? 200 : 500, {
    "Content-Type": "application/json",
    "Content-Encoding": "gzip",
    "Content-Length": compressed.length,
    "Cache-Control": "no-store",
  });
  res.end(compressed, () => { pendingRequests -= 1; });
});

function reservePort() {
  return new Promise((resolve, reject) => {
    const server = createTcpServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close((error) => error ? reject(error) : resolve(address.port));
    });
  });
}
async function stats() {
  const response = await fetch(apiUrl + "/__stats");
  assert.ok(response.ok);
  return response.json();
}
async function configure(imageCount) {
  const response = await fetch(apiUrl + "/__configure", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ image_count: imageCount }),
  });
  assert.ok(response.ok);
}
async function cdp(command, params = {}) {
  const response = await fetch("http://127.0.0.1:" + driverPort + "/session/" + browser.sessionId + "/goog/cdp/execute", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ cmd: command, params }),
  });
  assert.ok(response.ok, "CDP command failed: " + command);
  return (await response.json()).value;
}
function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}
function percentileDrop(before, after) {
  return before === 0 ? null : Number(((1 - after / before) * 100).toFixed(2));
}

await mkdir(artifacts, { recursive: true });
const report = {
  success: false,
  baselineRef,
  browser: null,
  conditions: { chapters: chapterCount, imageCounts, rounds, latencyMs: 50, downloadBytesPerSecond: 500000, cpuSlowdown: 4 },
  variants: {},
  samples: [],
  selectionChecks: [],
};
const tempRoot = path.resolve(root, "../..", ".tmp");
await mkdir(tempRoot, { recursive: true });
const temp = await mkdtemp(path.join(tempRoot, "manga-perf-"));
const previewServers = [];
let driver;
let browser;
let browserLog = "";
let apiUrl;
let driverPort;

try {
  await new Promise((resolve) => apiServer.listen(0, "127.0.0.1", resolve));
  apiUrl = "http://127.0.0.1:" + apiServer.address().port;
  const variants = [
    { name: "baseline", source: baselineSource },
    { name: "candidate", source: null },
  ];
  for (const variant of variants) {
    const outDir = path.join(artifacts, variant.name, "dist");
    await mkdir(path.dirname(outDir), { recursive: true });
    const plugins = variant.source == null ? [] : [{
      name: "manga-assets-baseline",
      enforce: "pre",
      load(id) { return id === mangaTabPath ? variant.source : null; },
    }];
    await build({
      root: frontend,
      logLevel: "warn",
      plugins,
      build: {
        outDir,
        emptyOutDir: true,
        rollupOptions: { input: path.join(frontend, "e2e/performance/manga-assets.html") },
      },
    });
    const port = await reservePort();
    const server = await preview({
      root: frontend,
      build: { outDir },
      preview: {
        host: "127.0.0.1",
        port,
        strictPort: true,
        proxy: { "/api": apiUrl },
      },
    });
    previewServers.push(server);
    variant.url = "http://127.0.0.1:" + port + "/e2e/performance/manga-assets.html";
    report.variants[variant.name] = { outDir, url: variant.url };
  }

  driverPort = await reservePort();
  driver = spawn(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", [
    "--port=" + driverPort,
    "--verbose",
    "--log-path=" + path.join(artifacts, "chromedriver.log"),
  ], {
    cwd: frontend,
    env: { ...process.env, TMPDIR: temp },
    stdio: ["ignore", "pipe", "pipe"],
  });
  for (const stream of [driver.stdout, driver.stderr]) stream.on("data", (chunk) => { browserLog = (browserLog + chunk).slice(-12000); });
  const driverDeadline = Date.now() + 30000;
  while (true) {
    assert.equal(driver.exitCode, null, browserLog);
    try {
      if ((await fetch("http://127.0.0.1:" + driverPort + "/status", { signal: AbortSignal.timeout(1000) })).ok) break;
    } catch {}
    assert.ok(Date.now() < driverDeadline, "ChromeDriver readiness timeout: " + browserLog);
    await delay(100);
  }
  browser = await remote({
    hostname: "127.0.0.1",
    port: driverPort,
    logLevel: "error",
    capabilities: {
      browserName: "chrome",
      "goog:chromeOptions": {
        binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium",
        args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", "--disable-features=BackForwardCache", "--enable-precise-memory-info", "--user-data-dir=" + path.join(temp, "chrome")],
      },
    },
  });
  report.browser = browser.capabilities.browserVersion;
  await browser.setTimeout({ script: 60000 });

  for (const imageCount of imageCounts) {
    for (let round = 0; round < rounds; round += 1) {
      const order = round % 2 === 0 ? ["baseline", "candidate"] : ["candidate", "baseline"];
      for (const name of order) {
        const variantInfo = report.variants[name];
        await browser.newWindow("about:blank", { type: "tab" });
        await browser.setWindowSize(1440, 900);
        await cdp("Network.enable");
        await cdp("Network.setCacheDisabled", { cacheDisabled: true });
        await cdp("Network.emulateNetworkConditions", {
          offline: false,
          latency: 50,
          downloadThroughput: 500000,
          uploadThroughput: 250000,
          connectionType: "cellular4g",
        });
        await cdp("Emulation.setCPUThrottlingRate", { rate: 4 });
        await cdp("Performance.enable");
        await cdp("HeapProfiler.enable");
        await configure(imageCount);
        await cdp("Network.clearBrowserCache");
        await browser.url(variantInfo.url + "?round=" + round + "&images=" + imageCount);
        await browser.waitUntil(
          async () => await browser.execute((count) => count === 1
            ? document.querySelector("section h2") != null
            : document.querySelectorAll('nav[aria-describedby="comic-chapter-list-scroll-hint"] li > button').length === count,
          chapterCount),
          { timeout: 30000, timeoutMsg: name + " chapter list did not render" },
        );
        assert.equal(
          await browser.execute(
            (count) => count === 1
              ? document.querySelector("section h2")?.textContent?.includes("第 1 章") === true
              : document.querySelector('nav[aria-describedby="comic-chapter-list-scroll-hint"] button[aria-current="true"]') != null,
            chapterCount,
          ),
          chapterCount === 1,
          name + " chapter auto-selection state",
        );
        const readyMs = await browser.execute(async () => {
          await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
          return performance.now();
        });
        const assetsExpectedInitially = name === "baseline" || chapterCount === 1;
        await browser.waitUntil(async () => {
          const current = await stats();
          const commands = new Set(current.requests.map((item) => item.command));
          const resources = await browser.execute(() => performance.getEntriesByType("resource").map((entry) => new URL(entry.name).pathname));
          return current.pending === 0 &&
            commands.has("list_manga_pipeline_statuses") &&
            (!assetsExpectedInitially || (
              commands.has("list_profiles") &&
              commands.has("list_portraits_by_project") &&
              commands.has("list_images_by_project")
            )) &&
            (name !== "baseline" || resources.some((resource) => resource.endsWith("/commands/list_images_by_project")));
        }, { timeout: 60000, timeoutMsg: name + " API fixture requests did not settle" });
        await delay(50);
        const resourceBytes = await browser.execute(() => performance.getEntriesByType("resource")
          .filter((entry) => new URL(entry.name).pathname.includes("/api/commands/"))
          .map((entry) => ({
            path: new URL(entry.name).pathname,
            decoded_bytes: entry.decodedBodySize,
            encoded_bytes: entry.encodedBodySize,
            duration_ms: Number(entry.duration.toFixed(2)),
          })));
        const pageText = await browser.execute(() => document.body.textContent ?? "");
        await cdp("HeapProfiler.collectGarbage");
        const heap = await cdp("Runtime.getHeapUsage");
        const currentStats = await stats();
        const commands = currentStats.requests.reduce((counts, request) => {
          counts[request.command] = (counts[request.command] ?? 0) + 1;
          return counts;
        }, {});
        const initialAssetRequests = ["list_profiles", "list_portraits_by_project", "list_images_by_project"]
          .reduce((total, command) => total + (commands[command] ?? 0), 0);
        assert.equal(initialAssetRequests, assetsExpectedInitially ? 3 : 0, name + " initial asset request count");
        assert.deepEqual(currentStats.unexpected, [], "Unexpected API requests: " + currentStats.unexpected.join(", "));
        const sample = {
          variant: name,
          image_count: imageCount,
          round,
          ready_ms: Number(readyMs.toFixed(2)),
          js_heap_used_bytes: heap.usedSize,
          api_decoded_bytes: currentStats.requests.reduce((total, request) => total + request.response_bytes, 0),
          api_wire_bytes: currentStats.requests.reduce((total, request) => total + request.response_wire_bytes, 0),
          initial_asset_requests: initialAssetRequests,
          commands,
          resources: resourceBytes,
          page_text_sha256: createHash("sha256").update(pageText).digest("hex"),
        };
        report.samples.push(sample);

        const isSelectionCheck = chapterCount > 1 &&
          imageCount === Math.max(...imageCounts) &&
          round === rounds - 1;
        if (isSelectionCheck) {
          const firstChapter = await browser.$('nav[aria-describedby="comic-chapter-list-scroll-hint"] li > button');
          await firstChapter.click();
          await browser.waitUntil(async () => await browser.execute(
            () => document.querySelector('nav[aria-describedby="comic-chapter-list-scroll-hint"] button[aria-current="true"]') != null,
          ), { timeout: 10000, timeoutMsg: name + " chapter selection did not apply" });
          await browser.waitUntil(async () => {
            const current = await stats();
            const commandsAfterSelect = current.requests.reduce((counts, request) => {
              counts[request.command] = (counts[request.command] ?? 0) + 1;
              return counts;
            }, {});
            return current.pending === 0 &&
              commandsAfterSelect.list_profiles === 1 &&
              commandsAfterSelect.list_portraits_by_project === 1 &&
              commandsAfterSelect.list_images_by_project === 1;
          }, { timeout: 60000, timeoutMsg: name + " did not load selected-chapter assets once" });
          await browser.waitUntil(async () => await browser.execute(
            () => performance.getEntriesByType("resource")
              .some((entry) => new URL(entry.name).pathname.endsWith("/commands/list_images_by_project")),
          ), { timeout: 60000, timeoutMsg: name + " image response did not reach the browser" });
          await browser.execute(async () => {
            await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
          });
          const selectedText = await browser.execute(() => document.body.textContent ?? "");
          report.selectionChecks.push({
            variant: name,
            image_count: imageCount,
            asset_requests: 3,
            selected_text_sha256: createHash("sha256").update(selectedText).digest("hex"),
          });
        }
        await browser.closeWindow();
      }
    }
  }

  report.summary = imageCounts.map((imageCount) => {
    const baseline = report.samples.filter((sample) => sample.variant === "baseline" && sample.image_count === imageCount);
    const candidate = report.samples.filter((sample) => sample.variant === "candidate" && sample.image_count === imageCount);
    assert.equal(candidate[0].page_text_sha256, baseline[0].page_text_sha256);
    const baselineReady = median(baseline.map((sample) => sample.ready_ms));
    const candidateReady = median(candidate.map((sample) => sample.ready_ms));
    const baselineHeap = median(baseline.map((sample) => sample.js_heap_used_bytes));
    const candidateHeap = median(candidate.map((sample) => sample.js_heap_used_bytes));
    const baselineBytes = median(baseline.map((sample) => sample.api_decoded_bytes));
    const candidateBytes = median(candidate.map((sample) => sample.api_decoded_bytes));
    return {
      image_count: imageCount,
      baseline_ready_p50_ms: baselineReady,
      candidate_ready_p50_ms: candidateReady,
      ready_reduction_percent: percentileDrop(baselineReady, candidateReady),
      baseline_js_heap_p50_bytes: baselineHeap,
      candidate_js_heap_p50_bytes: candidateHeap,
      js_heap_reduction_percent: percentileDrop(baselineHeap, candidateHeap),
      baseline_api_decoded_p50_bytes: baselineBytes,
      candidate_api_decoded_p50_bytes: candidateBytes,
      api_decoded_reduction_percent: percentileDrop(baselineBytes, candidateBytes),
    };
  });
  for (const summary of report.summary) {
    if (chapterCount >= 1000 && summary.image_count >= 5000) {
      assert.ok(
        summary.js_heap_reduction_percent >= 20 || summary.ready_reduction_percent >= 30,
        "Performance gate missed for " + summary.image_count + " images",
      );
    }
  }
  if (chapterCount > 1 && report.selectionChecks.length === 2) {
    assert.equal(report.selectionChecks[0].selected_text_sha256, report.selectionChecks[1].selected_text_sha256);
  }
  report.success = true;
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  for (const summary of report.summary) console.log(JSON.stringify(summary));
} catch (error) {
  report.error = String(error?.stack ?? error);
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  throw error;
} finally {
  if (browser) await browser.deleteSession();
  if (driver) await stopProcess(driver);
  for (const server of previewServers) await new Promise((resolve) => server.httpServer.close(resolve));
  if (apiServer.listening) await new Promise((resolve) => apiServer.close(resolve));
  await rm(temp, { recursive: true, force: true });
  await Promise.all(["baseline", "candidate"].map((name) => rm(path.join(artifacts, name, "dist"), { recursive: true, force: true })));
}
