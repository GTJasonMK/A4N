import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { build, preview } from "vite";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(frontend, "../data/performance-audit-20261015/asset-graph-layout/candidate"));
const port = Number(process.env.A4N_PERF_PORT ?? 48980);
const outDir = path.join(artifacts, "dist");
const baselineRef = process.env.A4N_PERF_BASELINE_REF;
const graphCanvasPath = path.join(frontend, "src/pages/novel-detail/asset-graph/graphCanvas.ts");
const baselineSource = baselineRef
  ? execFileSync("git", ["show", `${baselineRef}:frontend/src/pages/novel-detail/asset-graph/graphCanvas.ts`], { cwd: frontend, encoding: "utf8" })
  : null;
const tempRoot = path.resolve(process.env.A4N_PERF_TEMP_ROOT ?? path.join(frontend, "../../..", ".tmp", "asset-graph-browser"));
const chromiumBinary = process.env.A4N_PERF_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const browserVersion = execFileSync(chromiumBinary, ["--version"], { encoding: "utf8" }).trim();

await mkdir(artifacts, { recursive: true });
await mkdir(tempRoot, { recursive: true });
const temp = await mkdtemp(path.join(tempRoot, "run-"));
let server;
let browser;
let output = "";
let errors = "";
try {
  await build({
    root: frontend,
    logLevel: "warn",
    plugins: baselineSource == null ? [] : [{
      name: "asset-graph-performance-baseline",
      enforce: "pre",
      load: (id) => id === graphCanvasPath ? baselineSource : null,
    }],
    build: {
      outDir,
      emptyOutDir: true,
      rollupOptions: { input: path.join(frontend, "e2e/performance/asset-graph.html") },
    },
  });
  server = await preview({
    root: frontend,
    build: { outDir },
    preview: { host: "127.0.0.1", port, strictPort: true },
  });
  browser = spawn(chromiumBinary, [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--disable-background-networking",
    `--user-data-dir=${path.join(temp, "profile")}`,
    "--dump-dom",
    `http://127.0.0.1:${port}/e2e/performance/asset-graph.html?measure=1`,
  ], {
    cwd: frontend,
    env: { ...process.env, TMPDIR: temp },
    stdio: ["ignore", "pipe", "pipe"],
  });
  browser.stdout.on("data", (chunk) => { output += chunk; });
  browser.stderr.on("data", (chunk) => { errors += chunk; });
  const exit = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      browser.kill("SIGKILL");
      reject(new Error("Chromium asset graph benchmark timed out"));
    }, 60_000);
    browser.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    browser.once("exit", (code, signal) => {
      clearTimeout(timeout);
      if (code === 0) resolve({ code, signal });
      else reject(new Error(`Chromium exited with code=${code}, signal=${signal}: ${errors}`));
    });
  });
  await exit;
  const match = output.match(/<pre id="asset-graph-report">([^<]+)<\/pre>/);
  assert.ok(match, `Chromium did not return the benchmark report: ${output.slice(-1000)}\n${errors}`);
  const assetGraph = JSON.parse(match[1]);
  const report = { success: true, scenario: "asset-graph-layout", baselineRef: baselineRef ?? null, browser: browserVersion, assetGraph };
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
} catch (error) {
  const report = { success: false, state: "failed", baselineRef: baselineRef ?? null, error: String(error), chromium: errors };
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  throw error;
} finally {
  await stopProcess(browser);
  if (server) await new Promise((resolve, reject) => server.httpServer.close((error) => error ? reject(error) : resolve()));
  await rm(outDir, { recursive: true, force: true });
  await rm(temp, { recursive: true, force: true });
}
