import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { build, createServer as createViteServer } from "vite";
import { remote } from "webdriverio";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const workspace = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_SCHEMA_ARTIFACTS ?? path.join(workspace, "data/performance-audit-20260927/b02/schema-load"));
const baselineRef = process.env.A4N_PERF_SCHEMA_BASELINE ?? "0c83a6728";
const sourceFile = path.join(frontend, "src/api/generationEvents.ts");
const original = execFileSync("git", ["show", `${baselineRef}:frontend/src/api/generationEvents.ts`], { cwd: workspace, encoding: "utf8" });
const fixtures = path.join(workspace, "crates/process/a4n-service-protocol/fixtures/generation-events-v1");
const events = await Promise.all(["started.json", "text-delta-mixed-unicode.json", "text-delta-second-chunk.json"].map(async (name) => JSON.parse(await readFile(path.join(fixtures, name), "utf8"))));
await mkdir(artifacts, { recursive: true });
await mkdir(path.join(workspace, ".dev-runtime"), { recursive: true });
const temp = await mkdtemp(path.join(workspace, ".dev-runtime", "pschema-"));
const bundles = {};
const samples = [];
let driver;
let browser;
let devServer;
let driverLog = "";
const server = createServer((req, res) => {
  const name = req.url?.split("?")[0];
  if (name === "/") {
    res.writeHead(200, { "Content-Type": "text/html", "Cache-Control": "no-store" });
    res.end('<!doctype html><title>Generation protocol benchmark</title><link rel="icon" href="data:,"><p>Generation protocol benchmark</p>');
  } else if (bundles[name]) {
    res.writeHead(200, { "Content-Type": "text/javascript", "Cache-Control": "no-store" });
    res.end(bundles[name]);
  } else {
    res.writeHead(404).end();
  }
});

async function cdp(cmd, params) {
  const response = await fetch(`http://127.0.0.1:49012/session/${browser.sessionId}/goog/cdp/execute`, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ cmd, params }),
  });
  const body = await response.json();
  assert.ok(response.ok, JSON.stringify(body));
}

try {
  for (const variant of ["before", "after"]) {
    const outDir = path.join(artifacts, variant);
    await build({
      configFile: false, root: frontend, logLevel: "warn",
      define: { "process.env.NODE_ENV": '"production"' },
      plugins: [{ name: "schema-baseline", enforce: "pre", load: (id) => variant === "before" && id === sourceFile ? original : undefined }],
      build: { outDir, emptyOutDir: true, minify: true, lib: { entry: sourceFile, formats: ["es"], fileName: () => "protocol.mjs" } },
    });
    bundles[`/${variant}.mjs`] = await readFile(path.join(outDir, "protocol.mjs"));
  }
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const baseUrl = `http://127.0.0.1:${server.address().port}`;
  driver = spawn(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", ["--port=49012"], {
    cwd: frontend, env: { ...process.env, TMPDIR: temp }, stdio: ["ignore", "pipe", "pipe"],
  });
  for (const stream of [driver.stdout, driver.stderr]) stream.on("data", (data) => { driverLog += data; });
  const deadline = Date.now() + 30_000;
  while (true) {
    assert.equal(driver.exitCode, null, driverLog);
    try { if ((await fetch("http://127.0.0.1:49012/status")).ok) break; }
    catch (error) { if (error.name !== "TypeError") throw error; }
    assert.ok(Date.now() < deadline, driverLog);
    await delay(100);
  }
  browser = await remote({ hostname: "127.0.0.1", port: 49012, logLevel: "error", capabilities: {
    browserName: "chrome", "goog:chromeOptions": {
      binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium",
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(temp, "chrome")}`],
    },
  } });
  await cdp("Network.enable", {});
  await cdp("Network.setCacheDisabled", { cacheDisabled: true });
  await cdp("Emulation.setCPUThrottlingRate", { rate: 4 });
  for (let round = 0; round < 10; round++) {
    for (const variant of (round % 2 ? ["after", "before"] : ["before", "after"])) {
      await browser.url(`${baseUrl}/?round=${round}&variant=${variant}`);
      const result = await browser.execute(async (url, fixtures) => {
        const start = performance.now();
        const api = await import(url);
        const parsed = fixtures.map((event) => api.parseGenerationTaskEvent(event));
        const elapsedMs = performance.now() - start;
        for (let i = 0; i < 200; i++) fixtures.forEach(api.parseGenerationTaskEvent);
        const hotStart = performance.now();
        for (let i = 0; i < 1000; i++) fixtures.forEach(api.parseGenerationTaskEvent);
        const hotMs = performance.now() - hotStart;
        let error = null;
        try { api.parseGenerationTaskEvent({ ...fixtures[0], future_field: true }); }
        catch (failure) { error = failure.message; }
        return { elapsedMs, hotMs, parsed, error };
      }, `${baseUrl}/${variant}.mjs`, events);
      assert.deepEqual(result.parsed, events);
      assert.match(result.error, /additional properties/);
      if (round > 0) samples.push({ variant, round, elapsedMs: result.elapsedMs, hotMs: result.hotMs, error: result.error });
    }
  }
  const metrics = Object.fromEntries(["before", "after"].map((variant) => {
    const values = samples.filter((sample) => sample.variant === variant).map((sample) => sample.elapsedMs).sort((a, b) => a - b);
    const hot = samples.filter((sample) => sample.variant === variant).map((sample) => sample.hotMs).sort((a, b) => a - b);
    return [variant, { medianMs: values[4], hot3000EventsMs: hot[4], bytes: bundles[`/${variant}.mjs`].length }];
  }));
  assert.equal(new Set(samples.map((sample) => sample.error)).size, 1);
  devServer = await createViteServer({ root: frontend, cacheDir: path.join(artifacts, "dev-cache"), optimizeDeps: { entries: [] }, server: {
    host: "127.0.0.1", port: 49013, strictPort: true, hmr: false, watch: null, cors: { origin: baseUrl },
  } });
  await devServer.listen();
  await browser.url(baseUrl);
  const developmentResult = await browser.execute(async (fixtures) => {
    const api = await import("http://127.0.0.1:49013/src/api/generationEvents.ts");
    return fixtures.map((event) => api.parseGenerationTaskEvent(event));
  }, events);
  assert.deepEqual(developmentResult, events);
  const report = { success: true, baselineRef, cpuSlowdown: 4, metrics, samples, developmentModulePassed: true };
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(metrics));
} catch (error) {
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, error: String(error), samples }, null, 2));
  throw error;
} finally {
  await browser?.deleteSession();
  await devServer?.close();
  await stopProcess(driver);
  await new Promise((resolve) => server.close(resolve));
  await writeFile(path.join(artifacts, "chromedriver.log"), driverLog);
  await rm(temp, { recursive: true, force: true });
}
