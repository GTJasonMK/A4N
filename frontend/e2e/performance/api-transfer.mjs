import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer, request } from "node:http";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { gzipSync, gunzipSync } from "node:zlib";
import { remote } from "webdriverio";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const workspace = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(workspace, "data/performance-audit-20260927/b07/transfer"));
const dist = path.join(workspace, "data/performance-audit-20260927/b02/frontend-before");
const variants = [{ name: "before", binary: process.env.A4N_PERF_EDGE_BEFORE ?? path.join(workspace, "data/performance-audit-20260927/b06/frozen-final/a4n-edge") }];
if (process.env.A4N_PERF_EDGE_AFTER) variants.push({ name: "after", binary: process.env.A4N_PERF_EDGE_AFTER });
const rounds = Number(process.env.A4N_PERF_ROUNDS ?? 3);
const driverPort = Number(process.env.A4N_PERF_DRIVER_PORT ?? 49021);
assert.ok(Number.isInteger(rounds) && rounds > 0);
await mkdir(artifacts, { recursive: true });
const temp = await mkdtemp(path.join(workspace, ".dev-runtime", "api-transfer-"));
const sourcePaths = execFileSync("rg", ["--files", "prompts", "-g", "*.md"], { cwd: workspace, encoding: "utf8" }).trim().split("\n").sort();
const corpus = (await Promise.all(sourcePaths.map((file) => readFile(path.join(workspace, file), "utf8")))).join("\n");
const corpusBytes = Buffer.byteLength(corpus);
assert.ok(corpusBytes > 32768, "Corpus must exceed gzip's window instead of repeating a tiny phrase");
const payloads = Object.fromEntries([1, 10].map((mib) => [String(mib), Buffer.from(JSON.stringify({ content: corpus.repeat(Math.ceil(mib * 1024 * 1024 / corpusBytes)) }))]));
const digest = (value) => createHash("sha256").update(value).digest("hex");
const children = [];
const results = [];
let browser;
const control = createServer((req, res) => {
  req.resume();
  const url = new URL(req.url, "http://localhost");
  if (url.pathname === "/api/commands/oauth2_available_providers") {
    res.writeHead(200, { "Content-Type": "application/json" }); res.end("[]"); return;
  }
  if (req.headers.authorization !== "Bearer performance-fixture") {
    res.writeHead(401, { "Content-Type": "application/json" }); res.end('{"error":"missing fixture authorization"}'); return;
  }
  const body = payloads[url.searchParams.get("size") ?? "1"];
  if (url.pathname === "/api/performance/events") {
    res.writeHead(200, { "Content-Type": "text/event-stream" }); res.write("data: first\n\n");
    setTimeout(() => res.end("data: last\n\n"), 800); return;
  }
  if (url.pathname === "/api/performance/range") {
    res.writeHead(206, { "Content-Type": "application/json", "Content-Range": `bytes 0-4095/${body.length}` });
    res.end(body.subarray(0, 4096)); return;
  }
  const headers = { "Content-Type": "application/json; charset=utf-8", "Cache-Control": "no-store", "Vary": "origin" };
  if (url.pathname === "/api/performance/no-transform") headers["Cache-Control"] = "no-transform";
  if (url.pathname === "/api/performance/encoded") {
    const encoded = gzipSync(body); headers["Content-Encoding"] = "gzip";
    headers["Content-Length"] = String(encoded.length); res.writeHead(200, headers); res.end(encoded); return;
  }
  const data = url.pathname === "/api/performance/tiny" ? Buffer.from('{"ok":true}') : body;
  headers["Content-Length"] = String(data.length);
  res.writeHead(200, headers); res.end(data);
});

function launch(binary, args, env, name) {
  const child = spawn(binary, args, { cwd: workspace, env: { ...process.env, ...env, TMPDIR: temp }, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  child.stdout.on("data", (value) => { output += value; });
  child.stderr.on("data", (value) => { output += value; });
  children.push({ child, name, output: () => output });
  return child;
}

async function ready(child, probe) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    assert.equal(child.exitCode, null, children.find((entry) => entry.child === child)?.output());
    try { const value = await probe(); if (value) return value; }
    catch (error) { if (!["ENOENT", "ECONNREFUSED"].includes(error.code) && error.name !== "TypeError") throw error; }
    await delay(100);
  }
  throw new Error("Fixture process readiness timed out");
}

function raw(url, headers = {}, method = "GET") {
  return new Promise((resolve, reject) => {
    const started = performance.now(); let firstByteMs;
    const req = request(url, { method, headers: { Authorization: "Bearer performance-fixture", ...headers } }, (res) => {
      const chunks = [];
      res.on("data", (chunk) => { firstByteMs ??= performance.now() - started; chunks.push(chunk); });
      res.on("error", reject);
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers, body: Buffer.concat(chunks), firstByteMs, elapsedMs: performance.now() - started }));
    });
    req.setTimeout(10_000, () => req.destroy(new Error("HTTP smoke timeout")));
    req.on("error", reject); req.end();
  });
}

async function cdp(cmd, params = {}) {
  const response = await fetch(`http://127.0.0.1:${driverPort}/session/${browser.sessionId}/goog/cdp/execute`, {
    method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ cmd, params }),
  });
  const value = await response.json(); assert.ok(response.ok, JSON.stringify(value)); return value.value;
}

try {
  await new Promise((resolve) => control.listen(0, "127.0.0.1", resolve));
  for (const variant of variants) {
    const addressFile = path.join(temp, variant.name + ".addr");
    const child = launch(variant.binary, [], { A4N_FRONTEND_DIST: dist, A4N_CONTROL_ADDR: `127.0.0.1:${control.address().port}`,
      A4N_WEB_ADDR: "127.0.0.1:0", A4N_WEB_LISTEN_FILE: addressFile }, variant.name);
    variant.url = `http://${(await ready(child, () => readFile(addressFile, "utf8"))).trim()}`;
    const identity = await raw(`${variant.url}/api/performance/data`, { "Accept-Encoding": "identity" });
    const compressed = await raw(`${variant.url}/api/performance/data`, { "Accept-Encoding": "gzip" });
    assert.deepEqual(identity.body, payloads["1"]);
    assert.deepEqual(compressed.headers["content-encoding"] === "gzip" ? gunzipSync(compressed.body) : compressed.body, identity.body);
    if (variant.name === "after") {
      assert.equal(compressed.headers["content-encoding"], "gzip");
      assert.match(compressed.headers.vary, /accept-encoding/i); assert.match(compressed.headers.vary, /origin/i);
    }
    for (const endpoint of ["tiny", "no-transform", "range"]) {
      const response = await raw(`${variant.url}/api/performance/${endpoint}`, { "Accept-Encoding": "gzip" });
      assert.equal(response.headers["content-encoding"], undefined, endpoint);
      assert.deepEqual(response.body, endpoint === "tiny" ? Buffer.from('{"ok":true}') : endpoint === "range" ? payloads["1"].subarray(0, 4096) : payloads["1"]);
    }
    const disabled = await raw(`${variant.url}/api/performance/data`, { "Accept-Encoding": "gzip;q=0" });
    assert.equal(disabled.headers["content-encoding"], undefined); assert.deepEqual(disabled.body, identity.body);
    const head = await raw(`${variant.url}/api/performance/data`, { "Accept-Encoding": "gzip" }, "HEAD");
    assert.equal(head.status, 200); assert.equal(head.body.length, 0);
    const encoded = await raw(`${variant.url}/api/performance/encoded`, { "Accept-Encoding": "gzip" });
    assert.deepEqual(gunzipSync(encoded.body), payloads["1"]);
    const events = await raw(`${variant.url}/api/performance/events`, { "Accept-Encoding": "gzip" });
    assert.equal(events.headers["content-encoding"], undefined);
    assert.ok(events.elapsedMs - events.firstByteMs > 500, "SSE must arrive before its final event");
    variant.smoke = { success: true, bytes: identity.body.length, encodedBytes: compressed.body.length,
      sseFirstByteMs: events.firstByteMs, sseTotalMs: events.elapsedMs };
  }
  const driver = launch(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", [`--port=${driverPort}`, "--verbose", `--log-path=${path.join(artifacts, "driver-verbose.log")}`], {}, "driver");
  await ready(driver, async () => (await fetch(`http://127.0.0.1:${driverPort}/status`)).ok);
  browser = await remote({ hostname: "127.0.0.1", port: driverPort, logLevel: "error", capabilities: { browserName: "chrome",
    "goog:chromeOptions": { binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium",
      args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(temp, "chrome")}`] } } });
  await browser.setTimeout({ script: 60_000 });
  await cdp("Network.enable"); await cdp("Network.setCacheDisabled", { cacheDisabled: true });
  for (let round = 1; round <= rounds; round++) {
    for (const variant of (round % 2 ? variants : [...variants].reverse())) {
      await cdp("Network.emulateNetworkConditions", { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });
      await browser.url(variant.url);
      await browser.$('input[autocomplete="username"]').waitForExist({ timeout: 30_000 });
      await cdp("Network.emulateNetworkConditions", { offline: false, latency: 50, downloadThroughput: 500_000, uploadThroughput: 250_000 });
      for (const size of ["1", "10"]) {
        const measured = await browser.execute(async (size) => {
          const url = `/api/performance/data?size=${size}&nonce=${performance.now()}`;
          const started = performance.now();
          const response = await fetch(url, { headers: { Authorization: "Bearer performance-fixture" }, cache: "no-store" });
          if (!response.ok) throw new Error(`HTTP ${response.status}`);
          const text = await response.text(); const elapsedMs = performance.now() - started;
          JSON.parse(text);
          const hash = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text));
          const resource = performance.getEntriesByName(new URL(url, location.href).href).at(-1);
          return { elapsedMs, sha256: Array.from(new Uint8Array(hash), (v) => v.toString(16).padStart(2, "0")).join(""),
            encodedBytes: resource.encodedBodySize, decodedBytes: resource.decodedBodySize };
        }, size);
        assert.equal(measured.sha256, digest(payloads[size]));
        results.push({ variant: variant.name, round, size, ...measured });
        console.log(variant.name, size, round, measured.elapsedMs.toFixed(1), "ms", measured.encodedBytes, "bytes");
        await writeFile(path.join(artifacts, "partial.json"), JSON.stringify(results, null, 2));
      }
    }
  }
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: true, corpusBytes, corpusSha256: digest(corpus),
    payloadBytes: Object.fromEntries(Object.entries(payloads).map(([key, value]) => [key, value.length])),
    conditions: { latencyMs: 50, downloadBytesPerSecond: 500000, rounds }, variants, results }, null, 2));
} catch (error) {
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: false, error: String(error), results }, null, 2));
  throw error;
} finally {
  await browser?.deleteSession();
  for (const { child, name, output } of children.reverse()) { await stopProcess(child); await writeFile(path.join(artifacts, name + ".log"), output()); }
  await new Promise((resolve) => control.close(resolve));
  await rm(temp, { recursive: true, force: true });
}
