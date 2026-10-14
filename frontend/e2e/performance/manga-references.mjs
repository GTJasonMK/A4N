import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer as createTcpServer } from "node:net";
import path from "node:path";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";
import { build, preview } from "vite";
import { stopProcess } from "../process.mjs";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const root = path.resolve(frontend, "..");
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? path.join(root, "data/performance-audit-20261015/manga-references"));
const baselineRef = process.env.A4N_PERF_BASELINE_REF ?? "faab71131";
const imageCounts = (process.env.A4N_PERF_IMAGE_COUNTS ?? "5000,10000").split(",").map(Number);
const bindingCount = Number(process.env.A4N_PERF_BINDINGS ?? 3);
const profileCount = Number(process.env.A4N_PERF_PROFILE_COUNT ?? 50);
const referenceQuery = process.env.A4N_PERF_QUERY ?? null;
const rounds = Number(process.env.A4N_PERF_ROUNDS ?? 3);
const chromium = process.env.A4N_PERF_CHROMIUM_BINARY ?? "/usr/bin/chromium";
const sourceFiles = [
  "src/pages/novel-detail/manga/mangaPagePromptModel.ts",
  "src/pages/novel-detail/manga/ReferenceBindingsPanel.tsx",
];
assert.ok(imageCounts.length > 0 && imageCounts.every((count) => Number.isSafeInteger(count) && count > 0));
assert.ok(Number.isSafeInteger(bindingCount) && bindingCount > 0);
assert.ok(Number.isSafeInteger(profileCount) && profileCount >= 0);
assert.ok(Number.isSafeInteger(rounds) && rounds > 0);
const baselineSources = new Map(sourceFiles.map((file) => [
  path.join(frontend, file),
  execFileSync("git", ["show", baselineRef + ":frontend/" + file], { cwd: root, encoding: "utf8" }),
]));
const browserVersion = execFileSync(chromium, ["--version"], { encoding: "utf8" }).trim();

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
async function measure(chromiumUrl, temp, label, options = {}) {
  const width = options.width ?? 1440;
  const args = [
    "--headless=new",
    "--no-sandbox",
    "--disable-dev-shm-usage",
    "--disable-background-networking",
    "--enable-precise-memory-info",
    "--js-flags=--expose-gc",
    "--window-size=" + width + ",900",
    "--user-data-dir=" + path.join(temp, label),
  ];
  if (options.screenshotPath) args.push("--screenshot=" + options.screenshotPath);
  args.push("--dump-dom", chromiumUrl);
  const browser = spawn(chromium, args, {
    cwd: frontend,
    env: { ...process.env, TMPDIR: temp },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let output = "";
  let errors = "";
  browser.stdout.on("data", (chunk) => { output += chunk; });
  browser.stderr.on("data", (chunk) => { errors += chunk; });
  await new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      browser.kill("SIGKILL");
      reject(new Error("Chromium manga-reference benchmark timed out"));
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
  const match = output.match(/<pre id="manga-reference-report">([^<]+)<\/pre>/);
  assert.ok(match, "Chromium did not return a benchmark report: " + output.slice(-1000) + "\n" + errors);
  return JSON.parse(match[1]);
}

await mkdir(artifacts, { recursive: true });
const tempRoot = path.resolve(root, "../..", ".tmp", "manga-reference");
await mkdir(tempRoot, { recursive: true });
const temp = await mkdtemp(path.join(tempRoot, "run-"));
const previewServers = [];
const report = {
  success: false,
  scenario: "manga-reference-candidates",
  baselineRef,
  browser: browserVersion,
  conditions: { imageCounts, bindingCount, profileCount, referenceQuery, rounds },
  samples: [],
};
try {
  const variants = [
    { name: "baseline", sources: baselineSources },
    { name: "candidate", sources: null },
  ];
  for (const variant of variants) {
    const outDir = path.join(artifacts, variant.name, "dist");
    await mkdir(path.dirname(outDir), { recursive: true });
    const serverPort = await reservePort();
    await build({
      root: frontend,
      logLevel: "warn",
      plugins: variant.sources == null ? [] : [{
        name: "manga-reference-baseline",
        enforce: "pre",
        load: (id) => variant.sources.get(id),
      }],
      build: {
        outDir,
        emptyOutDir: true,
        rollupOptions: { input: path.join(frontend, "e2e/performance/manga-references.html") },
      },
    });
    const server = await preview({
      root: frontend,
      build: { outDir },
      preview: { host: "127.0.0.1", port: serverPort, strictPort: true },
    });
    previewServers.push(server);
    variant.url = "http://127.0.0.1:" + serverPort + "/e2e/performance/manga-references.html";
    report[variant.name + "OutDir"] = outDir;
  }

  for (const imageCount of imageCounts) {
    for (let round = 0; round < rounds; round += 1) {
      const order = round % 2 === 0 ? ["baseline", "candidate"] : ["candidate", "baseline"];
      for (const name of order) {
        const variant = variants.find((item) => item.name === name);
        assert.ok(variant);
        const result = await measure(
          variant.url + "?images=" + imageCount + "&bindings=" + bindingCount + "&profiles=" + profileCount +
            "&q=" + encodeURIComponent(referenceQuery ?? ("asset-" + imageCount)),
          temp,
          name + "-" + imageCount + "-" + round,
        );
        assert.equal(result.success, true, name + " option render failed: " + JSON.stringify(result));
        assert.equal(result.imageCount, imageCount);
        assert.equal(result.bindingCount, bindingCount);
        report.samples.push({ variant: name, round, ...result });
      }
    }
  }

  const candidateVariant = variants.find((item) => item.name === "candidate");
  const candidateCheck = await measure(
    candidateVariant.url + "?images=5000&bindings=" + bindingCount + "&profiles=" + profileCount +
      "&q=" + encodeURIComponent(referenceQuery ?? "asset-5000") + "&mode=measure",
    temp,
    "selected-reference-interaction",
  );
  assert.equal(candidateCheck.success, true, "Search result must be selectable");
  assert.equal(candidateCheck.selectedAddEnabled, true, "Selecting a search result must enable Add");
  assert.equal(candidateCheck.selectedPreviewVisible, true, "Selected image preview must be visible before Add");
  const keyboardCheck = await measure(
    candidateVariant.url + "?images=5000&bindings=" + bindingCount + "&profiles=" + profileCount +
      "&q=" + encodeURIComponent(referenceQuery ?? "asset-5000") + "&mode=keyboard",
    temp,
    "keyboard-reference-interaction",
  );
  assert.equal(keyboardCheck.success, true, "Keyboard search result must be selectable: " + JSON.stringify(keyboardCheck));
  assert.equal(keyboardCheck.selectedAddEnabled, true, "Keyboard selection must enable Add");
  assert.equal(keyboardCheck.selectedPreviewVisible, true, "Keyboard selection must show its image preview");
  report.interactionChecks = {
    pointerSelection: {
      optionCount: candidateCheck.optionCount,
      searchStatus: candidateCheck.searchStatus,
      selectedAddEnabled: candidateCheck.selectedAddEnabled,
      selectedPreviewVisible: candidateCheck.selectedPreviewVisible,
    },
    keyboardSelection: {
      optionCount: keyboardCheck.optionCount,
      searchStatus: keyboardCheck.searchStatus,
      selectedAddEnabled: keyboardCheck.selectedAddEnabled,
      selectedPreviewVisible: keyboardCheck.selectedPreviewVisible,
    },
  };

  await mkdir(path.join(artifacts, "screenshots"), { recursive: true });
  report.uiChecks = [];
  if (process.env.A4N_PERF_CAPTURE_UI !== "0") {
    for (const width of [1440, 1024, 390]) {
      for (const mode of ["default", "search", "selected"]) {
        const screenshotPath = path.join(artifacts, "screenshots", mode + "-" + width + ".png");
        const screenshotQuery = mode === "default" ? "" : referenceQuery ?? "asset-10000";
        const ui = await measure(
          candidateVariant.url + "?images=10000&bindings=" + bindingCount + "&profiles=" + profileCount +
            "&mode=" + mode + "&q=" + encodeURIComponent(screenshotQuery),
          temp,
          "ui-" + mode + "-" + width,
          { width, screenshotPath },
        );
        assert.equal(ui.success, true, mode + " UI did not render at " + width + "px");
        assert.equal(ui.overflow, 0, mode + " UI overflows at " + width + "px");
        report.uiChecks.push({
          width,
          mode,
          overflow: ui.overflow,
          optionCount: ui.optionCount,
          selectedPreviewVisible: ui.selectedPreviewVisible,
          screenshotPath,
        });
      }
    }
  }
  await writeFile(path.join(artifacts, "interaction-summary.json"), JSON.stringify({
    searchMode: candidateCheck.searchMode,
    optionCount: candidateCheck.optionCount,
    selectedAddEnabled: candidateCheck.selectedAddEnabled,
    searchStatus: candidateCheck.searchStatus,
  }, null, 2));

  report.summary = imageCounts.map((imageCount) => {
    const baseline = report.samples.filter((sample) => sample.variant === "baseline" && sample.imageCount === imageCount);
    const candidate = report.samples.filter((sample) => sample.variant === "candidate" && sample.imageCount === imageCount);
    const searchMode = imageCount + profileCount * 4 > 100;
    assert.equal(baseline[0].optionCount, imageCount + baseline[0].portraitCount + 1);
    assert.equal(candidate[0].searchMode, searchMode);
    if (searchMode) {
    assert.equal(candidate[0].searchQuery, referenceQuery ?? ("asset-" + imageCount));
    assert.ok(candidate[0].optionCount >= 2 && candidate[0].optionCount <= 101);
    } else {
      assert.equal(candidate[0].optionCount, baseline[0].optionCount);
    }
    const baselineMount = median(baseline.map((sample) => sample.mountMs));
    const candidateMount = median(candidate.map((sample) => sample.mountMs));
    const baselineHeap = median(baseline.map((sample) => sample.heapAfterMountBytes));
    const candidateHeap = median(candidate.map((sample) => sample.heapAfterMountBytes));
    const baselineOpen = median(baseline.map((sample) => sample.openMs));
    const candidateSearch = median(candidate.map((sample) => sample.searchMs));
    const candidateOpen = median(candidate.map((sample) => sample.openMs));
    const candidateInteraction = median(candidate.map((sample) => sample.searchMs + sample.openMs));
    const baselineOpenHeap = median(baseline.map((sample) => sample.heapAfterOpenBytes));
    const candidateOpenHeap = median(candidate.map((sample) => sample.heapAfterOpenBytes));
    return {
      image_count: imageCount,
      binding_count: bindingCount,
      baseline_option_count: baseline[0].optionCount,
      candidate_option_count: candidate[0].optionCount,
      baseline_mount_p50_ms: baselineMount,
      candidate_mount_p50_ms: candidateMount,
      mount_reduction_percent: reduction(baselineMount, candidateMount),
      baseline_heap_p50_bytes: baselineHeap,
      candidate_heap_p50_bytes: candidateHeap,
      heap_reduction_percent: reduction(baselineHeap, candidateHeap),
      baseline_open_p50_ms: baselineOpen,
      candidate_search_p50_ms: candidateSearch,
      candidate_open_p50_ms: candidateOpen,
      candidate_search_and_open_p50_ms: candidateInteraction,
      open_path_reduction_percent: reduction(baselineOpen, candidateInteraction),
      baseline_open_heap_p50_bytes: baselineOpenHeap,
      candidate_open_heap_p50_bytes: candidateOpenHeap,
    };
  });
  for (const summary of report.summary) {
    if (summary.image_count >= 5000 && bindingCount >= 3) {
      assert.ok(
        summary.mount_reduction_percent >= 30 ||
          summary.heap_reduction_percent >= 20 ||
          summary.open_path_reduction_percent >= 30,
        "Manga reference performance gate missed for " + summary.image_count + " images",
      );
    }
  }
  report.success = true;
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  for (const summary of report.summary) console.log(JSON.stringify(summary));
} catch (error) {
  report.error = String(error?.stack ?? error);
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  throw error;
} finally {
  for (const server of previewServers) await new Promise((resolve) => server.httpServer.close(resolve));
  await Promise.all(["baseline", "candidate"].map((name) => rm(path.join(artifacts, name, "dist"), { recursive: true, force: true })));
  await rm(temp, { recursive: true, force: true });
}
