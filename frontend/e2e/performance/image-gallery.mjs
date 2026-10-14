import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer as createTcpServer } from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { build, preview } from "vite";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const root = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(root, "data/performance-audit-20261015/image-gallery-batches"));
const baselineRef = process.env.A4N_PERF_BASELINE_REF ?? "56cb77fe2";
const imageCounts = (process.env.A4N_PERF_IMAGE_COUNTS ?? "1000,2000").split(",").map(Number);
const rounds = Number(process.env.A4N_PERF_ROUNDS ?? 3);
const chromium = process.env.A4N_PERF_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const uiOnly = process.env.A4N_PERF_UI_ONLY === "1";
const sourceFile = "src/pages/novel-detail/images/ImageGalleryPanel.tsx";
const baselineSource = execFileSync("git", ["show", baselineRef + ":frontend/" + sourceFile], {
  cwd: root,
  encoding: "utf8",
});
const svg = Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#84634e"/></svg>');
assert.ok(imageCounts.length > 0 && imageCounts.every((count) => Number.isSafeInteger(count) && count > 0));
assert.ok(Number.isSafeInteger(rounds) && rounds > 0);

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

function median(values) {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

function reduction(before, after) {
  return before === 0 ? null : Number(((1 - after / before) * 100).toFixed(2));
}

function cdpClient(socket) {
  let nextId = 0;
  const pending = new Map();
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id == null || !pending.has(message.id)) return;
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result);
  });
  return (method, params = {}, sessionId = undefined) => new Promise((resolve, reject) => {
    const id = ++nextId;
    pending.set(id, { resolve, reject });
    socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
  });
}

async function evaluate(cdp, sessionId, expression) {
  const result = await cdp("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true,
  }, sessionId);
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.text ?? "Browser evaluation failed");
  }
  return result.result.value;
}

const mediaPlugin = {
  name: "image-gallery-performance-media-fixture",
  enforce: "pre",
  load(id) {
    if (id === path.join(frontend, "src/utils/mediaSrc.ts")) {
      return "export function toMediaSrc(path) { return path || ''; }";
    }
  },
  configurePreviewServer(server) {
    server.middlewares.use((request, response, next) => {
      const url = new URL(request.url ?? "/", "http://localhost");
      if (!url.pathname.startsWith("/perf-media/")) return next();
      response.statusCode = 200;
      response.setHeader("Content-Type", "image/svg+xml");
      response.setHeader("Cache-Control", "no-store");
      response.setHeader("Content-Length", String(svg.byteLength));
      response.end(svg);
    });
  },
};

async function measure(url, temp, label, imageCount, options = {}) {
  const args = [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--enable-precise-memory-info",
    "--js-flags=--expose-gc",
    "--window-size=" + (options.width ?? 1440) + ",900",
    "--user-data-dir=" + path.join(temp, label),
    "--dump-dom",
  ];
  if (options.screenshotPath) args.push("--screenshot=" + options.screenshotPath);
  args.push(url + "?images=" + imageCount + "&run=" + encodeURIComponent(label) + (options.query ?? ""));
  const browser = spawn(chromium, args, { cwd: frontend, stdio: ["ignore", "pipe", "pipe"] });
  let output = "";
  let errors = "";
  browser.stdout.on("data", (chunk) => { output += chunk; });
  browser.stderr.on("data", (chunk) => { errors += chunk; });
  await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      browser.kill("SIGKILL");
      reject(new Error("Chromium image gallery benchmark timed out"));
    }, 90000);
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
  const match = output.match(/<pre id="image-gallery-report">([^<]+)<\/pre>/);
  assert.ok(match, "Chromium did not return an image gallery report: " + output.slice(-1000) + "\n" + errors);
  return JSON.parse(match[1]);
}

await mkdir(artifacts, { recursive: true });
const tempRoot = path.resolve(root, ".tmp/image-gallery-batches");
await mkdir(tempRoot, { recursive: true });
const temp = await mkdtemp(path.join(tempRoot, "run-"));
const previewServers = [];
let chromeProcess;
let cdpSocket;
let cdpCall;
let uiProfile;
const report = {
  success: false,
  scenario: "image-gallery-batched-rendering",
  baselineRef,
  browser: execFileSync(chromium, ["--version"], { encoding: "utf8" }).trim(),
  conditions: { imageCounts, rounds, imageSizeBytes: svg.byteLength, viewport: [1440, 900] },
  samples: [],
};

try {
  if (uiOnly) {
    const previous = JSON.parse(await readFile(path.join(artifacts, "report.json"), "utf8"));
    report.baselineRef = previous.baselineRef;
    report.conditions = previous.conditions;
    report.samples = previous.samples;
    report.summary = previous.summary;
  }
  const variants = uiOnly ? [
    { name: "candidate", load: () => undefined },
  ] : [
    { name: "baseline", load: (id) => id === path.join(frontend, sourceFile) ? baselineSource : undefined },
    { name: "candidate", load: () => undefined },
  ];
  for (const variant of variants) {
    const outDir = path.join(artifacts, variant.name, "dist");
    await mkdir(path.dirname(outDir), { recursive: true });
    await build({
      root: frontend,
      logLevel: "warn",
      plugins: [mediaPlugin, { name: variant.name + "-source", enforce: "pre", load: variant.load }],
      build: {
        outDir,
        emptyOutDir: true,
        rollupOptions: { input: path.join(frontend, "e2e/performance/image-gallery.html") },
      },
    });
    const port = await reservePort();
    const server = await preview({
      root: frontend,
      plugins: [mediaPlugin],
      build: { outDir },
      preview: { host: "127.0.0.1", port, strictPort: true },
    });
    previewServers.push(server);
    variant.url = "http://127.0.0.1:" + port + "/e2e/performance/image-gallery.html";
  }

  if (!uiOnly) {
    for (const imageCount of imageCounts) {
      for (let round = 0; round < rounds; round += 1) {
        const order = round % 2 === 0 ? ["baseline", "candidate"] : ["candidate", "baseline"];
        for (const name of order) {
          const variant = variants.find((item) => item.name === name);
          const result = await measure(
            variant.url,
            temp,
            name + "-" + imageCount + "-" + round,
            imageCount,
            { query: "&expand=all" },
          );
          assert.equal(result.count, imageCount);
          assert.equal(
            result.firstBatchImageCount,
            name === "baseline" ? imageCount : Math.min(imageCount, 500),
          );
          assert.equal(result.totalImageCount, imageCount);
          assert.equal(result.afterLoadMoreImageCount, imageCount);
          report.samples.push({ variant: name, round, ...result });
        }
      }
    }

    report.summary = imageCounts.map((imageCount) => {
      const summarize = (variant) => report.samples.filter((sample) =>
        sample.variant === variant && sample.count === imageCount);
      const baseline = summarize("baseline");
      const candidate = summarize("candidate");
      const baselineRequests = median(baseline.map((sample) => sample.requestedImageCount));
      const candidateRequests = median(candidate.map((sample) => sample.requestedImageCount));
      const baselineBytes = median(baseline.map((sample) => sample.requestedImageTransferBytes));
      const candidateBytes = median(candidate.map((sample) => sample.requestedImageTransferBytes));
      const baselineMount = median(baseline.map((sample) => sample.mountMs));
      const candidateMount = median(candidate.map((sample) => sample.mountMs));
      const baselineHeap = median(baseline.map((sample) => sample.heapAfterFirstBatchBytes));
      const candidateHeap = median(candidate.map((sample) => sample.heapAfterFirstBatchBytes));
      assert.ok(candidateRequests < baselineRequests, "Batched gallery must defer off-screen image cards");
      assert.ok(reduction(baselineBytes, candidateBytes) >= 30, "Image transfer reduction missed the performance gate");
      return {
        image_count: imageCount,
        baseline_rendered_count_p50: median(baseline.map((sample) => sample.firstBatchImageCount)),
        candidate_rendered_count_p50: median(candidate.map((sample) => sample.firstBatchImageCount)),
        baseline_requests_p50: baselineRequests,
        candidate_requests_p50: candidateRequests,
        request_reduction_percent: reduction(baselineRequests, candidateRequests),
        baseline_image_transfer_p50_bytes: baselineBytes,
        candidate_image_transfer_p50_bytes: candidateBytes,
        transfer_reduction_percent: reduction(baselineBytes, candidateBytes),
        baseline_mount_p50_ms: baselineMount,
        candidate_mount_p50_ms: candidateMount,
        mount_reduction_percent: reduction(baselineMount, candidateMount),
        baseline_heap_p50_bytes: baselineHeap,
        candidate_heap_p50_bytes: candidateHeap,
        heap_reduction_percent: reduction(baselineHeap, candidateHeap),
      };
    });
  }
  report.uiChecks = [];
  const candidate = variants.find((item) => item.name === "candidate");
  await mkdir(path.join(artifacts, "screenshots"), { recursive: true });
  const cdpTempRoot = path.resolve(root, "../../.tmp/ig");
  await mkdir(cdpTempRoot, { recursive: true });
  uiProfile = await mkdtemp(path.join(cdpTempRoot, "run-"));
  chromeProcess = spawn(chromium, [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--remote-debugging-port=0",
    "--user-data-dir=" + uiProfile,
  ], { cwd: frontend, env: { ...process.env, TMPDIR: cdpTempRoot }, detached: true, stdio: ["ignore", "ignore", "pipe"] });
  let chromeLog = "";
  chromeProcess.stderr.on("data", (data) => { chromeLog += data; });
  const devtoolsFile = path.join(uiProfile, "DevToolsActivePort");
  const deadline = Date.now() + 30000;
  let devtoolsPort;
  while (true) {
    assert.equal(chromeProcess.exitCode, null, chromeLog);
    try {
      const contents = await readFile(devtoolsFile, "utf8");
      devtoolsPort = Number(contents.trim().split("\n")[0]);
      if (Number.isSafeInteger(devtoolsPort)) break;
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    assert.ok(Date.now() < deadline, `Chromium DevTools readiness timeout: ${chromeLog}`);
    await delay(100);
  }
  const version = await fetch(`http://127.0.0.1:${devtoolsPort}/json/version`).then((response) => response.json());
  cdpSocket = new WebSocket(version.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    cdpSocket.addEventListener("open", resolve, { once: true });
    cdpSocket.addEventListener("error", reject, { once: true });
  });
  const cdp = cdpCall = cdpClient(cdpSocket);
  for (const width of [1440, 1024, 390]) {
    const target = await cdp("Target.createTarget", { url: "about:blank" });
    const attached = await cdp("Target.attachToTarget", { targetId: target.targetId, flatten: true });
    const sessionId = attached.sessionId;
    await cdp("Emulation.setDeviceMetricsOverride", {
      width,
      height: 900,
      deviceScaleFactor: 1,
      mobile: true,
    }, sessionId);
    await cdp("Page.enable", {}, sessionId);
    await cdp("Runtime.enable", {}, sessionId);
    await cdp("Page.navigate", {
      url: candidate.url + "?images=1000&run=ui-" + width,
    }, sessionId);
    const readyDeadline = Date.now() + 60000;
    let initial;
    while (Date.now() < readyDeadline) {
      await delay(100);
      initial = await evaluate(cdp, sessionId, `(() => {
        const report = document.getElementById("image-gallery-report")?.textContent ?? "";
        return {
          ready: report !== "" && report !== "running",
          viewportWidth: document.documentElement.clientWidth,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          corpusImageCount: Number(new URL(location.href).searchParams.get("images")),
          renderedImageCount: document.querySelectorAll('img[src^="/perf-media/"]').length,
          gallerySummary: [...document.querySelectorAll("p")]
            .map((item) => item.textContent?.trim() ?? "")
            .find((text) => text.startsWith("已显示 ")) ?? "",
          gallerySummaryTop: [...document.querySelectorAll("p")]
            .find((item) => item.textContent?.trim().startsWith("已显示 "))
            ?.getBoundingClientRect().top ?? null,
        };
      })()`);
      if (initial.ready) break;
    }
    assert.ok(initial?.ready, `Gallery did not finish loading at ${width}px: ${chromeLog}`);
    assert.equal(initial.viewportWidth, width);
    assert.equal(initial.overflow, 0);
    assert.equal(initial.corpusImageCount, 1000);
    assert.equal(initial.renderedImageCount, 500);
    assert.equal(initial.gallerySummary, "已显示 500/1,000 张图片");
    assert.ok(initial.gallerySummaryTop != null && initial.gallerySummaryTop >= 0 && initial.gallerySummaryTop < 900);
    const defaultScreenshot = path.join(artifacts, "screenshots", "default-" + width + ".png");
    const defaultImage = await cdp("Page.captureScreenshot", { format: "png", fromSurface: true }, sessionId);
    await writeFile(defaultScreenshot, Buffer.from(defaultImage.data, "base64"));

    const loadMoreTop = await evaluate(cdp, sessionId, `(() => {
      const scroller = document.getElementById("image-gallery-scroll");
      if (scroller) scroller.scrollTop = scroller.scrollHeight;
      const button = [...document.querySelectorAll("button")]
        .find((item) => item.textContent?.includes("再显示"));
      return button?.getBoundingClientRect().top ?? null;
    })()`);
    assert.ok(loadMoreTop != null && loadMoreTop <= 900);
    const bottomScreenshot = path.join(artifacts, "screenshots", "bottom-" + width + ".png");
    const bottomImage = await cdp("Page.captureScreenshot", { format: "png", fromSurface: true }, sessionId);
    await writeFile(bottomScreenshot, Buffer.from(bottomImage.data, "base64"));

    await evaluate(cdp, sessionId, `(() => {
      [...document.querySelectorAll("button")]
        .find((item) => item.textContent?.includes("再显示"))?.focus();
    })()`);
    await cdp("Input.dispatchKeyEvent", {
      type: "keyDown",
      key: "Enter",
      code: "Enter",
      windowsVirtualKeyCode: 13,
      nativeVirtualKeyCode: 13,
      text: "\r",
      unmodifiedText: "\r",
    }, sessionId);
    await cdp("Input.dispatchKeyEvent", {
      type: "keyUp",
      key: "Enter",
      code: "Enter",
      windowsVirtualKeyCode: 13,
      nativeVirtualKeyCode: 13,
    }, sessionId);
    const moreDeadline = Date.now() + 10000;
    let afterKeyboardMore = { renderedCount: 0, lastCardText: "" };
    while (Date.now() < moreDeadline) {
      afterKeyboardMore = await evaluate(cdp, sessionId, `(() => ({
        renderedCount: document.querySelectorAll("li").length,
        lastCardText: document.querySelector("li:last-child")?.textContent ?? "",
      }))()`);
      if (afterKeyboardMore.renderedCount === 1000) break;
      await delay(50);
    }
    assert.equal(afterKeyboardMore.renderedCount, 1000, `Keyboard activation failed at ${width}px`);
    assert.ok(afterKeyboardMore.lastCardText.includes("Image 1000"), `Last image was unavailable at ${width}px`);
    report.uiChecks.push({
      width,
      overflow: initial.overflow,
      corpusImageCount: initial.corpusImageCount,
      renderedBeforeMore: initial.renderedImageCount,
      gallerySummary: initial.gallerySummary,
      gallerySummaryTop: initial.gallerySummaryTop,
      loadMoreButtonTop: loadMoreTop,
      renderedAfterKeyboardMore: afterKeyboardMore.renderedCount,
      lastCardText: afterKeyboardMore.lastCardText,
      defaultScreenshot,
      bottomScreenshot,
    });
    await cdp("Target.closeTarget", { targetId: target.targetId });
  }
  report.success = true;
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  for (const summary of report.summary) console.log(JSON.stringify(summary));
} catch (error) {
  report.error = String(error?.stack ?? error);
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  throw error;
} finally {
  if (cdpCall && chromeProcess?.exitCode == null) {
    await Promise.race([
      cdpCall("Browser.close").catch(() => undefined),
      delay(1000),
    ]);
  }
  cdpSocket?.close();
  if (chromeProcess) {
    try { process.kill(-chromeProcess.pid, "SIGTERM"); } catch {}
    await Promise.race([
      new Promise((resolve) => chromeProcess.once("exit", resolve)),
      delay(5000),
    ]);
    if (chromeProcess.exitCode == null) {
      try { process.kill(-chromeProcess.pid, "SIGKILL"); } catch {}
      await new Promise((resolve) => chromeProcess.once("exit", resolve));
    }
  }
  if (uiProfile) await rm(uiProfile, { recursive: true, force: true });
  for (const server of previewServers) await new Promise((resolve) => server.httpServer.close(resolve));
  await Promise.all(["baseline", "candidate"].map((name) => rm(path.join(artifacts, name, "dist"), { recursive: true, force: true })));
  await rm(temp, { recursive: true, force: true });
}
