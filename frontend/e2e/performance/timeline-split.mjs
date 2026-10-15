import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer as createTcpServer } from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build, preview } from "vite";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const root = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(root, "data/performance-audit-20261015/timeline-split"));
const pointCounts = (process.env.A4N_PERF_POINT_COUNTS ?? "1000,5000").split(",").map(Number);
const chromium = process.env.A4N_PERF_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const outDir = path.join(artifacts, "dist");
assert.ok(pointCounts.length > 0 && pointCounts.every((count) => Number.isSafeInteger(count) && count > 1));

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

async function measure(url, temp, label, pointCount) {
  const browser = spawn(chromium, [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--user-data-dir=" + path.join(temp, label),
    "--dump-dom",
    url + "?points=" + pointCount,
  ], { cwd: frontend, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  let errors = "";
  browser.stdout.on("data", (chunk) => { output += chunk; });
  browser.stderr.on("data", (chunk) => { errors += chunk; });
  await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      browser.kill("SIGKILL");
      reject(new Error("Chromium timeline-split benchmark timed out"));
    }, 60000);
    browser.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    browser.once("exit", (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) resolve(undefined);
      else reject(new Error("Chromium exited with code=" + code + ", signal=" + signal + ": " + errors));
    });
  });
  const match = output.match(/<pre id="timeline-split-report">([^<]+)<\/pre>/);
  assert.ok(match, "Chromium returned no timeline split report: " + errors);
  return JSON.parse(match[1]);
}

await mkdir(artifacts, { recursive: true });
const tempRoot = path.resolve(root, ".tmp/timeline-split");
await mkdir(tempRoot, { recursive: true });
const temp = await mkdtemp(path.join(tempRoot, "run-"));
let server;
try {
  const port = await reservePort();
  await build({
    root: frontend,
    logLevel: "warn",
    build: {
      outDir,
      emptyOutDir: true,
      rollupOptions: { input: path.join(frontend, "e2e/performance/timeline-split.html") },
    },
  });
  server = await preview({
    root: frontend,
    build: { outDir },
    preview: { host: "127.0.0.1", port, strictPort: true },
  });
  const url = "http://127.0.0.1:" + port + "/e2e/performance/timeline-split.html";
  const results = [];
  for (const pointCount of pointCounts) {
    const result = await measure(url, temp, "points-" + pointCount, pointCount);
    assert.equal(result.success, true);
    assert.equal(result.pointCount, pointCount);
    assert.ok(result.reductionPercent >= 30, "Split option projection missed the performance gate");
    assert.ok(result.segmentLookup.reductionPercent >= 30, "Segment lookup missed the performance gate");
    results.push(result);
  }
  const browserVersion = execFileSync(chromium, ["--version"], { encoding: "utf8" }).trim();
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify({ success: true, browser: browserVersion, results }, null, 2));
  for (const result of results) console.log(JSON.stringify(result));
} finally {
  if (server) await new Promise((resolve) => server.httpServer.close(resolve));
  await Promise.all([rm(outDir, { recursive: true, force: true }), rm(temp, { recursive: true, force: true })]);
}
