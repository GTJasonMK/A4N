import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer, request } from "node:http";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { gunzipSync } from "node:zlib";
import { remote } from "webdriverio";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const workspace = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_STARTUP_ARTIFACTS ?? path.join(workspace, "data/performance-audit-20260927/b02/startup"));
const dist = path.resolve(process.env.A4N_PERF_STARTUP_DIST ?? path.join(workspace, "data/performance-audit-20260927/b02/frontend-before"));
const before = process.env.A4N_PERF_EDGE_BEFORE ?? path.join(workspace, "data/performance-audit-20260927/b02/edge-before");
const variants = [{ name: "before", binary: before, compressed: false }];
if (process.env.A4N_PERF_EDGE_AFTER) variants.push({ name: "after", binary: process.env.A4N_PERF_EDGE_AFTER, compressed: true });
const samples = Number(process.env.A4N_PERF_STARTUP_SAMPLES ?? 9);
assert.ok(Number.isInteger(samples) && samples > 0);
const driverPort = Number(process.env.A4N_PERF_DRIVER_PORT ?? 49011);
await mkdir(artifacts, { recursive: true });
await mkdir(path.join(workspace, ".dev-runtime"), { recursive: true });
const temp = await mkdtemp(path.join(workspace, ".dev-runtime", "pstart-"));
const children = [];
const requests = [];
const results = [];
let browser;
const control = createServer((req, res) => {
  requests.push(req.url);
  req.resume();
  if (req.url === "/api/commands/oauth2_available_providers") {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end("[]");
  } else {
    res.writeHead(404, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ error: `Unexpected fixture request: ${req.url}` }));
  }
});

function launch(binary, args, extraEnv, name) {
  const child = spawn(binary, args, {
    cwd: workspace, env: { ...process.env, ...extraEnv, TMPDIR: temp },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  child.stdout.on("data", (data) => { output += data; });
  child.stderr.on("data", (data) => { output += data; });
  child.on("error", (error) => { output += String(error); });
  children.push({ child, name, output: () => output });
  return child;
}

function rawRequest(url, headers = {}, method = "GET") {
  return new Promise((resolve, reject) => {
    const req = request(url, { headers, method }, (res) => {
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("error", reject);
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(chunks) }));
    });
    req.setTimeout(10_000, () => req.destroy(new Error(`Request timed out: ${url}`)));
    req.on("error", reject);
    req.end();
  });
}

async function waitFor(child, probe) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    assert.equal(child.exitCode, null, children.find((entry) => entry.child === child)?.output());
    try { const value = await probe(); if (value) return value; }
    catch (error) { if (!["ENOENT", "ECONNREFUSED"].includes(error.code) && error.name !== "TypeError") throw error; }
    await delay(100);
  }
  throw new Error("Test process did not become ready");
}

async function cdp(cmd, params = {}) {
  const response = await fetch(`http://127.0.0.1:${driverPort}/session/${browser.sessionId}/goog/cdp/execute`, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ cmd, params }),
  });
  const value = await response.json();
  assert.ok(response.ok, JSON.stringify(value));
  return value.value;
}

async function checkRepresentations(variant, asset) {
  const url = `${variant.url}/${asset}`;
  const original = await readFile(path.join(dist, asset));
  const plain = await rawRequest(url, { "Accept-Encoding": "identity" });
  const encoded = await rawRequest(url, { "Accept-Encoding": "gzip" });
  assert.equal(plain.status, 200);
  assert.deepEqual(plain.body, original);
  assert.equal(encoded.status, 200);
  assert.equal(encoded.headers["content-type"], plain.headers["content-type"]);
  if (variant.compressed) {
    assert.equal(encoded.headers["content-encoding"], "gzip");
    assert.deepEqual(gunzipSync(encoded.body), original);
    for (const response of [plain, encoded]) assert.match(response.headers.vary, /accept-encoding/i);
  } else {
    assert.equal(encoded.headers["content-encoding"], undefined);
    assert.deepEqual(encoded.body, original);
  }
  const head = await rawRequest(url, { "Accept-Encoding": "gzip" }, "HEAD");
  assert.equal(head.body.length, 0);
  assert.equal(Number(head.headers["content-length"]), encoded.body.length);
  const rejectedEncoding = await rawRequest(url, { "Accept-Encoding": "gzip;q=0" });
  assert.equal(rejectedEncoding.headers["content-encoding"], undefined);
  assert.deepEqual(rejectedEncoding.body, original);
  const range = await rawRequest(url, { "Accept-Encoding": "identity", Range: "bytes=0-31" });
  assert.equal(range.status, 206);
  assert.deepEqual(range.body, original.subarray(0, 32));
  const unchanged = await rawRequest(url, { "Accept-Encoding": "gzip", "If-Modified-Since": encoded.headers["last-modified"] });
  assert.equal(unchanged.status, 304);
  const fallback = await rawRequest(`${variant.url}/route-that-is-handled-by-the-client`, { "Accept-Encoding": "gzip" });
  const index = await readFile(path.join(dist, "index.html"));
  assert.deepEqual(variant.compressed ? gunzipSync(fallback.body) : fallback.body, index);
  const api = await rawRequest(`${variant.url}/api/commands/oauth2_available_providers`, { "Accept-Encoding": "gzip" });
  assert.equal(api.headers["content-encoding"], undefined);
  assert.equal(api.body.toString(), "[]");
  return { rawBytes: original.length, wireBytes: encoded.body.length, contentType: encoded.headers["content-type"] };
}

try {
  await new Promise((resolve) => control.listen(0, "127.0.0.1", resolve));
  for (const variant of variants) {
    const listenFile = path.join(temp, `${variant.name}.addr`);
    const edge = launch(variant.binary, [], {
      A4N_FRONTEND_DIST: dist, A4N_CONTROL_ADDR: `127.0.0.1:${control.address().port}`,
      A4N_WEB_ADDR: "127.0.0.1:0", A4N_WEB_LISTEN_FILE: listenFile,
    }, `edge-${variant.name}`);
    const address = await waitFor(edge, () => readFile(listenFile, "utf8"));
    variant.url = `http://${address.trim()}`;
    const manifest = JSON.parse(await readFile(path.join(dist, ".vite/manifest.json"), "utf8"));
    variant.delivery = await checkRepresentations(variant, manifest["index.html"].file);
  }
  const driver = launch(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", [`--port=${driverPort}`], {}, "chromedriver");
  await waitFor(driver, async () => (await fetch(`http://127.0.0.1:${driverPort}/status`)).ok);
  browser = await remote({ hostname: "127.0.0.1", port: driverPort, logLevel: "error", capabilities: {
    browserName: "chrome", "goog:chromeOptions": {
      binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium",
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(temp, "chrome")}`],
    },
  } });
  await cdp("Network.enable");
  await cdp("Network.setCacheDisabled", { cacheDisabled: true });
  await cdp("Emulation.setCPUThrottlingRate", { rate: 4 });
  await cdp("Network.emulateNetworkConditions", { offline: false, latency: 50, downloadThroughput: 500_000, uploadThroughput: 250_000, connectionType: "cellular4g" });
  await cdp("Page.addScriptToEvaluateOnNewDocument", { source: `
    window.__a4nStartup = { ready: null, lcp: null };
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) window.__a4nStartup.lcp = entry.startTime;
    }).observe({ type: "largest-contentful-paint", buffered: true });
    const observer = new MutationObserver(() => {
      const username = document.querySelector('input[autocomplete="username"]');
      const password = document.querySelector('input[type="password"]');
      if (username && password && !username.disabled && !password.disabled) {
        observer.disconnect();
        requestAnimationFrame(() => requestAnimationFrame(() => { window.__a4nStartup.ready = performance.now(); }));
      }
    });
    observer.observe(document, { childList: true, subtree: true });
  ` });
  for (const width of [1440, 390]) {
    await browser.setViewport({ width, height: 900 });
    for (let round = 0; round <= samples; round++) {
      for (const variant of (round % 2 ? [...variants].reverse() : variants)) {
        await cdp("Network.clearBrowserCache");
        await browser.url(`${variant.url}/?perf-round=${round}`);
        await browser.waitUntil(() => browser.execute(() => window.__a4nStartup?.ready != null), { timeout: 30_000 });
        await delay(300);
        const measured = await browser.execute(() => ({
          ...window.__a4nStartup,
          fcp: performance.getEntriesByName("first-contentful-paint")[0]?.startTime,
          resources: performance.getEntriesByType("resource").map(({ name, encodedBodySize, decodedBodySize, transferSize }) => ({ name: new URL(name).pathname, encodedBodySize, decodedBodySize, transferSize })),
          text: document.querySelector("main").textContent,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        }));
        assert.equal(measured.overflow, 0, `${variant.name} overflows at ${width}px`);
        measured.textSha256 = createHash("sha256").update(measured.text).digest("hex");
        delete measured.text;
        if (round > 0) results.push({ variant: variant.name, width, round, ...measured });
        if (round === samples) {
          await browser.$('input[autocomplete="username"]').click();
          await browser.keys("performance-fixture");
          await browser.keys("Tab");
          assert.equal(await browser.execute(() => document.activeElement.type), "password");
          await browser.keys("not-a-real-password");
          assert.equal(await browser.$('button[type="submit"]').isEnabled(), true);
          await browser.saveScreenshot(path.join(artifacts, `${variant.name}-${width}.png`));
        }
        console.log(variant.name, width, round, measured.ready.toFixed(1), "ms");
      }
    }
  }
  assert.ok(requests.every((url) => url === "/api/commands/oauth2_available_providers"), JSON.stringify(requests));
  const report = { success: true, dist, conditions: { latencyMs: 50, bytesPerSecond: 500_000, cpuSlowdown: 4, samples }, variants, results };
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
} catch (error) {
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, error: String(error), requests, results }, null, 2));
  throw error;
} finally {
  await browser?.deleteSession();
  for (const { child, name, output } of children.reverse()) {
    await stopProcess(child);
    await writeFile(path.join(artifacts, `${name}.log`), output());
  }
  await new Promise((resolve) => control.close(resolve));
  await rm(temp, { recursive: true, force: true });
}
