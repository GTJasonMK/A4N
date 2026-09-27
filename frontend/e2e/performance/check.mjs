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
const baselineSources = new Map(baselineRef ? ["src/hooks/usePacedStreamText.ts", "src/i18n/format.ts", "src/components/ui/MarkdownText.tsx"].map((file) => [
  path.join(frontend, file),
  execFileSync("git", ["show", `${baselineRef}:frontend/${file}`], { cwd: frontend, encoding: "utf8" }),
]) : []);
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, state: "running" }));
const tempRoot = path.join(frontend, "../.dev-runtime");
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
    rollupOptions: { input: path.join(frontend, "e2e/performance/index.html") },
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
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(temp, "chrome")}`],
    },
  } });
  await browser.setTimeout({ script: 60_000 });
  await browser.url(`http://127.0.0.1:${port}/e2e/performance/`);
  await browser.waitUntil(() => browser.execute(() => window.performanceReady === true));
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
  const report = { success: true, baselineRef: baselineRef ?? null, browser: browser.capabilities.browserVersion, viewports: [1440, 390], trace: traces[0], streams, formatting, markdown };
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
