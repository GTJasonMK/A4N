import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { remote } from "webdriverio";

const frontend = fileURLToPath(new URL("../..", import.meta.url));
const artifacts = path.resolve(process.env.A4N_BROWSER_E2E_ARTIFACT_DIR ?? path.join(frontend, "../data/director-ui"));
const temporary = path.resolve(process.env.TMPDIR ?? path.join(artifacts, "tmp"));
await mkdir(temporary, { recursive: true });
await mkdir(artifacts, { recursive: true });
const environment = { ...process.env, TMPDIR: temporary };
const port = 43871;
const driverPort = 43872;
const savedStatus = "导演编排草稿已保存";
const processes = [];
let browser;
const logs = [];
const measurements = [];
const launch = (command, args) => {
  const child = spawn(command, args, { cwd: frontend, env: environment, stdio: ["ignore", "pipe", "pipe"] });
  child.stdout.on("data", (chunk) => logs.push(String(chunk)));
  child.stderr.on("data", (chunk) => logs.push(String(chunk)));
  processes.push(child);
  return child;
};
async function ready(child, url) {
  const until = Date.now() + 30_000;
  while (Date.now() < until) {
    if (child.exitCode != null) throw new Error(`Child exited: ${logs.join("")}`);
    try { if ((await fetch(url)).ok) return; } catch { /* process is starting */ }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Server readiness timeout: ${url}`);
}
async function setViewport(width, height) {
  await browser.setViewport({ width, height });
  await browser.waitUntil(async () => browser.execute((width, height) => innerWidth === width && innerHeight === height, width, height));
}
async function openSelect(label) {
  const trigger = await browser.$(`button[role="combobox"][aria-label="${label}"]`);
  await trigger.waitForDisplayed();
  await trigger.click();
  await browser.$('[role="listbox"]').waitForDisplayed();
  return trigger;
}
async function assertWorkspaceFits(state) {
  const measurement = await browser.execute(() => {
    const workspace = document.querySelector(".director-workspace");
    const inspector = document.querySelector(".director-inspector");
    const track = document.querySelector(".director-timeline-scroll").getBoundingClientRect();
    const blockedTimelineTargets = [...document.querySelectorAll(".director-point, .director-segment, .director-segment-boundary")].filter((node) => {
      const rect = node.getBoundingClientRect();
      const x = rect.left + rect.width / 2;
      const y = rect.top + rect.height / 2;
      return x >= track.left && x <= track.right && y >= track.top && y <= track.bottom && !node.contains(document.elementFromPoint(x, y));
    }).map((node) => node.getAttribute("aria-label"));
    const containers = [];
    for (let node = workspace; node; node = node.parentElement) {
      containers.push({ className: node.className, x: node.scrollWidth - node.clientWidth, y: node.scrollHeight - node.clientHeight });
    }
    return {
      width: innerWidth, height: innerHeight,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      inspectorOverflow: inspector ? inspector.scrollHeight - inspector.clientHeight : 0,
      blockedTimelineTargets,
      containers,
      regions: [".director-header", ".director-tool-rail", ".director-viewer", ".director-transport", ".director-timeline"].map((selector) => {
        const { top, bottom, left, right, height } = document.querySelector(selector).getBoundingClientRect();
        return { selector, top, bottom, left, right, height };
      }),
    };
  });
  measurements.push({ state, ...measurement });
  assert.equal(measurement.overflow, 0, `${state}: document must not overflow horizontally`);
  assert.ok(measurement.inspectorOverflow <= 1, `${state}: default inspector controls must fit without scrolling`);
  assert.deepEqual(measurement.blockedTimelineTargets, [], `${state}: points, clips and boundaries must each remain clickable in the shared track`);
  for (const container of measurement.containers) {
    assert.ok(container.x <= 1 && container.y <= 1, `${state}: page container must not scroll: ${JSON.stringify(container)}`);
  }
  for (const region of measurement.regions) {
    assert.ok(region.height > 0 && region.top >= 0 && region.bottom <= measurement.height + 1 && region.left >= 0 && region.right <= measurement.width + 1, `${state}: region must stay on screen: ${JSON.stringify(region)}`);
    if (region.selector === ".director-viewer") assert.ok(region.height >= 100, `${state}: preview must remain usable, not shrink to a thumbnail`);
    // The inspector can make the timeline narrow even in a desktop viewport.
    if (region.selector === ".director-timeline") assert.ok(region.height <= (region.right - region.left >= 768 ? 180 : 240), `${state}: timeline must leave room for the preview`);
  }
}
async function assertStageNavigation(width) {
  const compact = width < 1280;
  const selector = await browser.$("#video-stages-3-select");
  const tabs = await browser.$(".a4n-media-stage-tabs nav");
  assert.equal(await selector.isDisplayed(), compact, `${width}px: compact stage selector visibility`);
  assert.equal(await tabs.isDisplayed(), !compact, `${width}px: wide stage tabs visibility`);
  const scrollableTimeline = await browser.execute(() => {
    const timeline = document.querySelector(".director-timeline-scroll");
    return timeline.scrollWidth > timeline.clientWidth + 1;
  });
  assert.equal(await browser.$(".director-timeline-scroll-hint").isDisplayed(), scrollableTimeline,
    `${width}px: a scrollable timeline must explain horizontal scrolling`);
  if (scrollableTimeline) {
    const scroll = await browser.execute(() => {
      const timeline = document.querySelector(".director-timeline-scroll");
      const start = timeline.scrollLeft;
      timeline.scrollLeft = timeline.clientWidth;
      const moved = timeline.scrollLeft > start;
      timeline.scrollLeft = start;
      return { moved, clientWidth: timeline.clientWidth, scrollWidth: timeline.scrollWidth };
    });
    assert.ok(scroll.moved && scroll.scrollWidth > scroll.clientWidth,
      `${width}px: later timeline ranges must remain reachable`);
  }
  if (width === 1024) {
    await selector.click();
    await browser.keys(["End", "Enter"]);
    assert.match(await selector.getText(), /后期处理/, "the compact selector must expose the last stage");
    await selector.click();
    await browser.keys(["Home", "ArrowDown", "Enter"]);
    assert.match(await selector.getText(), /导演编排/, "the active stage must remain directly selectable");
  }
  return { width, compact, scrollableTimeline };
}
try {
  const vite = launch(process.execPath, ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", String(port), "--strictPort"]);
  const driver = launch(process.env.A4N_WDIO_CHROMEDRIVER_BINARY ?? "/usr/bin/chromedriver", [`--port=${driverPort}`, "--verbose", `--log-path=${path.join(artifacts, "driver.log")}`]);
  await Promise.all([ready(vite, `http://127.0.0.1:${port}/e2e/director-timeline/`), ready(driver, `http://127.0.0.1:${driverPort}/status`)]);
  browser = await remote({ hostname: "127.0.0.1", port: driverPort, logLevel: "error", capabilities: {
    browserName: "chrome", "goog:chromeOptions": { binary: process.env.A4N_WDIO_CHROMIUM_BINARY ?? "/usr/bin/chromium", args: ["--headless=new", "--no-sandbox", "--disable-dev-shm-usage", `--user-data-dir=${path.join(artifacts, "chrome")}`, "--window-size=1440,1000"] },
  } });
  await setViewport(1440, 900);
  await browser.url(`http://127.0.0.1:${port}/e2e/director-timeline/`);
  await browser.$(".director-workspace").waitForExist({ timeout: 20_000 });
  assert.equal((await browser.$$(".director-point")).length, 7);
  assert.equal((await browser.$$(".director-segment")).length, 3);
  const frame = await browser.$(".director-viewer").getSize();
  assert.ok(Math.abs(frame.width / frame.height - 16 / 9) < 0.02);
  await assertWorkspaceFits("initial");
  await browser.$(".director-inspector").waitForDisplayed();
  await browser.saveScreenshot(path.join(artifacts, "initial.png"));
  await browser.$('[aria-label^="图片点 2s"]').click();
  await setInput('[aria-label="播放位置"]', "3000");
  const description = await browser.$(".director-fields textarea");
  const originalDescription = await description.getValue();
  const originalLabel = await browser.$(".director-fields input").getValue();
  const initialSaves = await browser.execute(() => window.directorCommands.filter((item) => item.command === "save_chapter_visual_plan_draft").length);
  await description.setValue("仅修改一次，保存回执会调整 JSON 字段顺序");
  await browser.waitUntil(async () => (await browser.$(".director-save-status").getText()) === savedStatus, {
    timeout: 6000,
    timeoutMsg: "字段顺序变化不应让已保存草稿持续处于未保存状态",
  });
  const stableSave = await browser.executeAsync((done) => {
    const button = [...document.querySelectorAll(".director-inline-actions button")].find((item) => item.textContent.includes("确认视觉编排"));
    const saveCount = () => window.directorCommands.filter((item) => item.command === "save_chapter_visual_plan_draft").length;
    const before = saveCount();
    let disabledChanges = 0;
    const observer = new MutationObserver((mutations) => { disabledChanges += mutations.length; });
    observer.observe(button, { attributes: true, attributeFilter: ["disabled"] });
    setTimeout(() => {
      observer.disconnect();
      done({ before, after: saveCount(), disabledChanges, disabled: button.disabled,
        connected: button.isConnected, status: document.querySelector(".director-save-status").textContent });
    }, 2400);
  });
  assert.equal(stableSave.before, initialSaves + 1, "one edit should produce exactly one save");
  assert.equal(stableSave.after, stableSave.before, "idle saved plans must not be saved repeatedly");
  assert.equal(stableSave.disabledChanges, 0, "confirmation must not flicker between enabled and disabled");
  assert.equal(stableSave.disabled, false);
  assert.equal(stableSave.connected, true);
  assert.equal(stableSave.status, savedStatus);
  measurements.push({ state: "saved-confirmation-stability", ...stableSave });
  await browser.saveScreenshot(path.join(artifacts, "stable-confirmation.png"));
  await description.setValue(originalDescription);
  await browser.waitUntil(async () => (await browser.$(".director-save-status").getText()) === savedStatus);
  const initialReads = await browser.execute(() => window.directorCommands.filter((item) => item.command === "get_chapter_visual_plan").length);
  await browser.execute(() => {
    window.nativeWorkspace = document.querySelector(".director-workspace");
    window.nativeSelected = document.querySelector(".director-point.is-selected");
    window.holdDirectorSave();
  });
  await description.setValue("第一版人工描述");
  await browser.waitUntil(async () => browser.execute(() => window.directorSaveHeld === true));
  await description.setValue("保存期间继续输入，必须保留");
  await browser.execute(() => {
    window.emitDirectorPointChange("p1", { label: "Agent 原生回执", at_ms: 2500 });
    window.releaseDirectorSave();
  });
  await browser.waitUntil(async () => (await browser.$(".director-fields input").getValue()) === "Agent 原生回执");
  assert.equal(await description.getValue(), "保存期间继续输入，必须保留");
  assert.equal(await browser.$('[aria-label="播放位置"]').getValue(), "3000", "remote edits never seek the playhead");
  assert.deepEqual(await browser.execute(() => ({
    workspace: window.nativeWorkspace === document.querySelector(".director-workspace"),
    selection: window.nativeSelected === document.querySelector(".director-point.is-selected"),
    focus: document.activeElement === document.querySelector(".director-fields textarea"),
  })), { workspace: true, selection: true, focus: true });
  assert.equal(await browser.execute(() => window.directorCommands.filter((item) => item.command === "get_chapter_visual_plan").length), initialReads, "receipt application must not reload the editor");
  await browser.saveScreenshot(path.join(artifacts, "native-receipt-preserves-editor.png"));
  await browser.$('[aria-label="撤销"]').click();
  assert.equal(await browser.$(".director-fields input").getValue(), originalLabel);
  assert.equal(await description.getValue(), "保存期间继续输入，必须保留", "undo of a remote batch retains manual input");
  await description.setValue(originalDescription);
  await browser.waitUntil(async () => (await browser.$(".director-save-status").getText()) === savedStatus);
  await description.setValue("本地冲突字段");
  await browser.execute(() => window.emitDirectorPointChange("p1", { description: "远端冲突字段", label: "非冲突字段合并保留" }));
  await browser.$("button=冲突字段保留我的编辑").waitForDisplayed();
  assert.equal(await description.getValue(), "本地冲突字段");
  await browser.saveScreenshot(path.join(artifacts, "native-conflict.png"));
  await browser.$("button=冲突字段保留我的编辑").click();
  assert.equal(await description.getValue(), "本地冲突字段");
  assert.equal(await browser.$(".director-fields input").getValue(), "非冲突字段合并保留");
  await description.setValue(originalDescription);
  await browser.$(".director-fields input").setValue(originalLabel);
  await browser.waitUntil(async () => (await browser.$(".director-save-status").getText()) === savedStatus);
  for (const width of [1024, 390]) {
    await setViewport(width, 844);
    if (width === 390) await browser.$('[aria-label="属性"]').click();
    const localDescription = await browser.$(".director-fields textarea");
    await localDescription.setValue("窄屏本地冲突");
    await browser.execute(() => window.emitDirectorPointChange("p1", { description: "窄屏远端版本" }));
    const compare = await browser.$(".director-conflict-details");
    await compare.waitForDisplayed();
    assert.match(await compare.getText(), /窄屏本地冲突/);
    assert.match(await compare.getText(), /窄屏远端版本/);
    await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((item) => item.command === "control_agent_run" && item.input.action === "pause")));
    await browser.saveScreenshot(path.join(artifacts, `native-conflict-${width}.png`));
    await compare.click();
    await browser.keys("Tab");
    await browser.keys("Tab");
    assert.match(await browser.execute(() => document.activeElement.textContent), /采用已保存版本/);
    await browser.keys("Enter");
    await compare.waitForExist({ reverse: true });
    assert.equal(await localDescription.getValue(), "窄屏远端版本");
    await localDescription.setValue(originalDescription);
    if (width === 390) await browser.keys("Escape");
    await browser.waitUntil(async () => (await browser.$(".director-save-status").getText()) === savedStatus);
  }
  await setViewport(1440, 900);
  await browser.$('[aria-label^="固定端点 0s"]').click();
  await browser.$('[aria-label="导演 Agent"]').click();
  await browser.$(".director-agent-inspector").waitForDisplayed();
  await browser.$(".agent-message-collapsible").waitForDisplayed();
  assert.equal(await browser.$(".agent-message-collapsible").getAttribute("open"), null, "long Agent replies stay collapsed until requested");
  await browser.$(".agent-console-stop-action").waitForDisplayed();
  assert.equal(await browser.$(".agent-console-stop-action").getAttribute("aria-label"), "取消");
  assert.equal(await browser.$("[aria-label=\"发送给领域 Agent 的消息\"]").isExisting(), false, "an active Run uses the send slot for stop");
  await browser.saveScreenshot(path.join(artifacts, "agent-compact.png"));
  const agentPanel = await browser.execute(() => ({
    dialogs: document.querySelectorAll('[role="dialog"]').length,
    inspector: document.querySelector(".director-agent-inspector")?.getBoundingClientRect().toJSON() ?? null,
    console: document.querySelector(".director-agent-console")?.getBoundingClientRect().toJSON() ?? null,
  }));
  assert.equal(agentPanel.dialogs, 0, "desktop director Agent must use the inspector, not a dialog");
  assert.ok(agentPanel.inspector && agentPanel.console && Math.abs(agentPanel.inspector.height - agentPanel.console.height) <= 1, "director Agent must fill the right inspector");
  await browser.$('.director-agent-console [aria-label="关闭"]').click();
  await browser.$(".director-inspector").waitForDisplayed({ reverse: true });
  await browser.$('[aria-label="属性"]').click();
  await browser.$(".director-inspector").waitForDisplayed();
  await browser.$('[aria-label="原文"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  assert.equal(await browser.$(".director-inspector").isDisplayed(), true, "opening a dialog must not replace the desktop inspector");
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.$(".director-inspector").waitForDisplayed();
  function setInput(selector, value) { return browser.execute((selector, value) => {
    const input = document.querySelector(selector);
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  }, selector, value); }
  await setInput('[aria-label="播放位置"]', "3000");
  await browser.waitUntil(async () => (await browser.$(".director-frame-caption").getText()).includes("2s"));
  await setInput('[aria-label="播放位置"]', "5000");
  await browser.waitUntil(async () => (await browser.$(".director-frame-caption").getText()).includes("4s"));
  await browser.$('[aria-label="播放分镜"]').click();
  await browser.waitUntil(async () => (await browser.$(".director-timecode").getText()).includes("5.") || (await browser.$(".director-frame-caption").getText()).includes("6s"));
  await browser.$('[aria-label="暂停"]').click();
  await browser.saveScreenshot(path.join(artifacts, "desktop.png"));
  await browser.$$(".director-segment")[0].click();
  const prompt = await browser.$(".director-inspector textarea");
  await prompt.setValue("镜头缓缓推近，旅人在门前停步。");
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "save_chapter_visual_plan_draft" && call.input.plan.segments[0].prompt.includes("缓缓推近"))));
  await browser.saveScreenshot(path.join(artifacts, "desktop-properties.png"));
  const sceneMode = await openSelect("场景方式");
  await browser.keys("Escape");
  const timeBeforeSelectKeys = await browser.$(".director-timecode").getText();
  for (const key of ["End", "Home"]) {
    await browser.keys(key);
    assert.equal(await browser.$(".director-timecode").getText(), timeBeforeSelectKeys, "focused selects must not seek the timeline");
  }
  await sceneMode.click();
  await browser.keys(["ArrowDown", "Enter"]);
  await openSelect("目标场景");
  await browser.keys(["End", "ArrowUp", "Enter"]);
  await openSelect("起始场景");
  await browser.keys(["ArrowDown", "Enter"]);
  await openSelect("目标场景");
  await browser.keys(["ArrowUp", "Enter"]);
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => {
    const binding = call.command === "save_chapter_visual_plan_draft" && call.input.plan.segments[0].scene_binding;
    return binding?.kind === "transition" && binding.from_scene_profile_id === 22 && binding.to_scene_profile_id === 11;
  })), { timeoutMsg: "themed scene selections must reach the saved plan" });
  await openSelect("目标场景");
  await browser.keys(["End", "Enter"]);
  await browser.$(".director-custom-scene input").waitForDisplayed();
  await browser.$(".director-custom-scene").$("button=取消").click();
  assert.equal(await browser.$('button[role="combobox"][aria-label="目标场景"]').getText(), "雨夜街道");
  await openSelect("场景方式");
  await browser.keys(["Home", "Enter"]);
  await openSelect("场景");
  await browser.keys(["Home", "ArrowDown", "Enter"]);
  await browser.$$(".director-point")[1].click();
  await browser.$("button=分镜候选 (2)").click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await browser.$(".director-point-candidate-grid").waitForDisplayed();
  assert.equal((await browser.$$(".director-point-candidate")).length, 2, "one image point must retain multiple generated candidates");
  assert.equal((await browser.$$("button=切换绑定为此候选图")).length, 2);
  await browser.$$("button=切换绑定为此候选图")[0].click();
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "save_chapter_visual_plan_draft" && call.input.plan.points[1].image_id === 9)));
  assert.equal((await browser.$$("button=当前绑定候选图")).length, 1, "an image point can bind only one candidate");
  assert.equal((await browser.$$("button=切换绑定为此候选图")).length, 1, "the other candidate must remain switchable");
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0, "candidate dialog must not overflow the document");
  await browser.saveScreenshot(path.join(artifacts, "desktop-candidates.png"));
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await assertWorkspaceFits("candidate grid");
  await browser.$$(".director-point")[0].click();
  await browser.$('[aria-label="收起属性"]').click();
  await browser.$('[aria-label="调整整体时长"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await setInput('[aria-label^="整体时长"]', "24");
  await browser.$('[aria-label^="整体时长"]').click();
  await browser.keys("Tab");
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.duration_ms === 24000)));
  await browser.waitUntil(async () => (await browser.$(".director-ruler").getText()).includes("24s"));
  await browser.$("button=完成").click();
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.$('[aria-label="调整整体时长"]').click();
  await browser.$('[aria-label^="整体时长"]').setValue("30");
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  assert.equal(await browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.duration_ms === 30000)), false, "Escape must cancel an uncommitted duration edit");
  assert.match(await browser.$('[aria-label="调整整体时长"]').getText(), /24s/);
  for (const [width, height] of [[1440, 900], [1366, 768], [1280, 720], [1024, 768], [900, 844], [390, 844], [390, 667]]) {
    await setViewport(width, height);
    await browser.execute(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    if (width >= 1024 && !await browser.$(".director-inspector").isExisting()) await browser.$('[aria-label="属性"]').click();
    await assertWorkspaceFits(`${width}x${height}`);
    measurements.push({ state: "stage navigation", ...(await assertStageNavigation(width)) });
    await browser.saveScreenshot(path.join(artifacts, `viewport-${width}x${height}.png`));
  }
  await setInput('[aria-label="播放位置"]', "3000");
  await browser.$('[aria-label="调整整体时长"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  assert.equal(await browser.$('[aria-label^="整体时长"]').getValue(), "24");
  assert.ok((await browser.$('[aria-label="等比伸缩全部时间点"]').getSize()).height >= 44);
  const timeBeforeTimingKeys = await browser.$(".director-timecode").getText();
  await browser.keys("Home");
  assert.equal(await browser.$(".director-timecode").getText(), timeBeforeTimingKeys);
  await browser.saveScreenshot(path.join(artifacts, "mobile-timing.png"));
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.$$(".director-point")[1].click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await browser.waitUntil(async () => browser.execute(() => getComputedStyle(document.querySelector('[role="dialog"]')).opacity === "1"));
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  const mediaActionsLayout = await browser.execute(() => {
    const actions = document.querySelector(".director-mobile-inspector .director-point-media-actions");
    const body = actions?.closest('[data-modal-body="1"]');
    const sizeSelect = document.querySelector('.director-mobile-inspector .director-fields [role="combobox"]');
    const candidates = [...document.querySelectorAll(".director-mobile-inspector .director-fields button")]
      .find((button) => button.textContent.includes("分镜候选"));
    const footer = document.querySelector(".director-mobile-inspector > .a4n-dialog-section:last-child");
    if (!actions || !body || !sizeSelect || !candidates || !footer) return null;
    const actionRect = actions.getBoundingClientRect();
    const sizeRect = sizeSelect.getBoundingClientRect();
    const candidateRect = candidates.getBoundingClientRect();
    const bodyRect = body.getBoundingClientRect();
    const footerTop = footer.getBoundingClientRect().top;
    return {
      width: innerWidth,
      height: innerHeight,
      actionTop: actionRect.top,
      actionBottom: actionRect.bottom,
      bodyTop: bodyRect.top,
      bodyBottom: bodyRect.bottom,
      footerTop,
      fullyVisible: actionRect.top >= bodyRect.top && actionRect.bottom <= bodyRect.bottom && actionRect.bottom <= footerTop,
      sizeSelectTop: sizeRect.top,
      sizeSelectBottom: sizeRect.bottom,
      sizeSelectFullyVisible: sizeRect.top >= bodyRect.top && sizeRect.bottom <= bodyRect.bottom && sizeRect.bottom <= footerTop,
      candidatesTop: candidateRect.top,
      candidatesBottom: candidateRect.bottom,
      candidatesFullyVisible: candidateRect.top >= bodyRect.top && candidateRect.bottom <= bodyRect.bottom && candidateRect.bottom <= footerTop,
    };
  });
  measurements.push({ state: "mobile media actions", ...mediaActionsLayout });
  assert.equal(mediaActionsLayout?.fullyVisible, true, "primary media actions must be fully visible above the mobile footer");
  assert.equal(mediaActionsLayout?.sizeSelectFullyVisible, true, "the generation size selector must be fully visible above the mobile footer");
  assert.equal(mediaActionsLayout?.candidatesFullyVisible, true, "the candidate entry must be fully visible above the mobile footer");
  await browser.saveScreenshot(path.join(artifacts, "mobile-properties.png"));
  const imageSizeSelect = await browser.$('.director-fields [role="combobox"]');
  await imageSizeSelect.waitForDisplayed();
  assert.equal(await imageSizeSelect.isDisplayed(), true, "image property section must expose a size selector");
  assert.match(await imageSizeSelect.getText(), /1920\s*x\s*1080/);
  await browser.$("button=重新生成").click();
  await browser.$(".director-generation").waitForDisplayed();
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "start_image_generation")), { timeout: 10_000, timeoutMsg: "regeneration must start immediately" });
  assert.equal(await browser.$("button=开始生成").isExisting(), false, "regeneration must not require a second start action");
  assert.equal(await browser.$(".director-generation-panel").isExisting(), false, "image generation must not open a redundant lower panel");
  assert.equal(await browser.$("h4=生成分镜图片").isExisting(), false, "image generation must stay in the image properties section");
  assert.equal(await browser.$('.director-generation [role="combobox"]').isExisting(), false, "generation panel must not duplicate the image size selector");
  assert.equal(await browser.$(".director-generation img").isExisting(), false, "generation panel must not duplicate the generated preview");
  assert.equal(await browser.$(".director-generation").$("button=采用候选").isExisting(), false, "image candidates are adopted from the candidate dialog");
  assert.equal((await browser.$$('[role="dialog"]')).length, 1, "image generation must stay in the properties dialog");
  assert.equal(await browser.$("h3=图片点属性").isDisplayed(), true, "generation must not replace the properties heading");
  assert.equal(await browser.$(".director-generation").isDisplayed(), true, "generation progress must render in the properties panel");
  assert.equal(await browser.$(".director-generation textarea").isExisting(), false, "generation must not ask for another prompt");
  const generationStart = await browser.execute(() => window.directorCommands.find((call) => call.command === "start_image_generation"));
  assert.equal(generationStart.input.operation.options.config_id, null, "generation must use the configured system default image model");
  assert.equal(generationStart.input.operation.options.user_description, "旅人在雨夜走近旧书店。", "generation must reuse the point description");
  assert.deepEqual(generationStart.input.operation.options.size, { width: 1920, height: 1080 }, "generation must use the selected image size");
  const stopGeneration = await browser.$("button=停止生成");
  const stopRect = await browser.execute((button) => {
    const rect = button.getBoundingClientRect();
    return { top: rect.top, bottom: rect.bottom };
  }, stopGeneration);
  assert.ok(stopRect.top >= 0 && stopRect.bottom <= 667, "generation cancel action must remain reachable in the mobile dialog");
  const mobileFooterTop = await browser.execute(() => document.querySelector(".director-mobile-inspector > .a4n-dialog-section:last-child")?.getBoundingClientRect().top ?? 667);
  assert.ok(stopRect.bottom <= mobileFooterTop, "generation cancel action must not sit behind the mobile footer");
  await browser.saveScreenshot(path.join(artifacts, "mobile-generation.png"));
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.$('[aria-label="属性"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await browser.$("h3=图片点属性").waitForDisplayed();
  const timeBeforeDialogKeys = await browser.$(".director-timecode").getText();
  await browser.keys("Home");
  assert.equal(await browser.$(".director-timecode").getText(), timeBeforeDialogKeys);
  const splitButton = await browser.$("button=从此处分镜");
  await splitButton.scrollIntoView({ block: "center" });
  assert.ok(await splitButton.isClickable());
  const reachable = await browser.execute((button) => {
    const rect = button.getBoundingClientRect();
    return rect.top >= 0 && rect.bottom <= innerHeight;
  }, splitButton);
  assert.ok(reachable, "mobile inspector bottom actions must be fully visible after scrolling");
  await splitButton.click();
  await browser.$("h3=拆分镜头").waitForDisplayed();
  const splitPosition = await openSelect("分镜位置");
  await browser.keys(["End", "Enter"]);
  assert.match(await splitPosition.getText(), /门前停步/);
  await openSelect("分镜位置");
  await browser.saveScreenshot(path.join(artifacts, "mobile-split-select.png"));
  await browser.keys("Escape");
  await browser.$('[role="listbox"]').waitForExist({ reverse: true });
  assert.equal(await browser.$("h3=拆分镜头").isDisplayed(), true, "Escape must close the select before its dialog");
  await browser.keys("Escape");
  await browser.$("h3=放弃拆分镜头编辑？").waitForDisplayed();
  await browser.$("button=放弃草稿").click();
  await browser.$("h3=拆分镜头").waitForDisplayed({ reverse: true });
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.$('[aria-label="原文"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  assert.equal(await browser.$('[aria-label="原文"]').getAttribute("aria-haspopup"), "dialog");
  assert.equal(await browser.$('[aria-label="原文"]').getAttribute("aria-expanded"), "true", "the open action must expose its dialog state");
  assert.equal(await browser.$('[aria-label="原文"]').getAttribute("aria-pressed"), null, "dialog actions must not look like tabs");
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  assert.equal(await browser.$('[aria-label="原文"]').getAttribute("aria-expanded"), "false", "closing a dialog must clear its expanded state");
  await browser.$('[aria-label="放大视图"]').click();
  assert.ok((await browser.$(".director-timeline-track").getSize()).width >= 640);
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  await browser.saveScreenshot(path.join(artifacts, "mobile-expanded.png"));
  await setViewport(1440, 900);
  await browser.$$(".director-point")[0].click();
  for (const [label, ratio] of [["9:16 竖屏", 9 / 16], ["1:1 方形", 1], ["4:3", 4 / 3], ["16:9 横屏", 16 / 9]]) {
    await browser.$('[aria-label="画幅"]').click();
    assert.equal(await browser.$('[aria-label="画幅"]').getAttribute("aria-haspopup"), "dialog");
    assert.equal(await browser.$('[aria-label="画幅"]').getAttribute("aria-expanded"), "true", "the open aspect-ratio action must expose its dialog state");
    assert.equal(await browser.$('[aria-label="画幅"]').getAttribute("aria-pressed"), null, "dialog actions must not look like tabs");
    await browser.$(`button=${label}`).click();
    await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
    assert.equal(await browser.$('[aria-label="画幅"]').getAttribute("aria-expanded"), "false", "closing the aspect-ratio dialog must clear its expanded state");
    await browser.waitUntil(async () => {
      const frame = await browser.$(".director-viewer").getSize();
      return Math.abs(frame.width / frame.height - ratio) < 0.02;
    });
    await assertWorkspaceFits(label);
    await browser.saveScreenshot(path.join(artifacts, `frame-${label.split(" ")[0].replace(":", "-")}.png`));
  }
  await browser.execute(() => { document.querySelector("main").style.width = "700px"; });
  await browser.$(".director-inspector").waitForExist({ reverse: true });
  assert.equal(await browser.$('[role="dialog"]').isExisting(), false, "narrowing the workspace must not open a dialog automatically");
  await assertWorkspaceFits("narrow chapter workspace on desktop");
  await browser.$('[aria-label="属性"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await browser.keys("Escape");
  await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
  await browser.execute(() => { document.querySelector("main").style.width = ""; });
  await browser.$(".director-inspector").waitForDisplayed();
  await setViewport(1366, 768);
  await browser.execute(() => { document.querySelector("main").style.padding = "60px 12px 44px"; });
  await browser.$$(".director-segment")[1].click();
  await assertWorkspaceFits("host chrome and transition properties");
  await browser.saveScreenshot(path.join(artifacts, "host-chrome.png"));
  await browser.execute(() => { document.querySelector("main").style.padding = ""; });
  for (const width of [1440, 1024, 390]) {
    await setViewport(width, 844);
    await browser.$$(".director-segment")[1].click();
    await openSelect("目标场景");
    const popup = await browser.execute(() => {
      const listbox = document.querySelector('[role="listbox"]');
      const { left, right, top, bottom } = listbox.getBoundingClientRect();
      return { left, right, top, bottom, overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth };
    });
    measurements.push({ state: "scene select popup", width, ...popup });
    assert.equal(popup.overflow, 0);
    assert.ok(popup.left >= 0 && popup.right <= width && popup.top >= 0 && popup.bottom <= 844, "scene options must remain inside the viewport");
    await browser.saveScreenshot(path.join(artifacts, `scene-select-${width}.png`));
    await browser.keys("Escape");
    await browser.$('[role="listbox"]').waitForExist({ reverse: true });
    if (width === 390) {
      assert.equal(await browser.$('[role="dialog"]').isDisplayed(), true);
      await browser.keys("Escape");
      await browser.$('[role="dialog"]').waitForDisplayed({ reverse: true });
    }
  }
  await setViewport(1440, 900);
  await browser.execute(() => window.setDirectorCanEdit(false));
  await browser.waitUntil(async () => !await browser.$(".director-inspector textarea").isEnabled());
  assert.equal(await browser.$("button=添加图片点").isEnabled(), false);
  for (const select of await browser.$$('.director-scene-binding button[role="combobox"]')) {
    assert.equal(await select.isEnabled(), false, "read-only scene selectors must stay disabled");
  }
  await browser.execute(() => window.setDirectorCanEdit(true));
  const center = (element) => browser.execute((node) => { const r = node.getBoundingClientRect(); return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }; }, element);
  const drag = async (start, end) => {
    await browser.performActions([{ type: "pointer", id: "timeline-mouse", parameters: { pointerType: "mouse" }, actions: [
      { type: "pointerMove", duration: 0, ...start }, { type: "pointerDown", button: 0 },
      { type: "pointerMove", duration: 350, ...end }, { type: "pointerUp", button: 0 },
    ] }]);
    await browser.releaseActions();
  };
  const movingPoint = await browser.$$(".director-point")[1];
  await movingPoint.scrollIntoView({ block: "center" });
  const pointCenter = await center(movingPoint);
  await drag(pointCenter, { x: pointCenter.x + 40, y: pointCenter.y });
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.kind === "move_point" && call.input.operation.point_id === "p1" && call.input.operation.at_ms > 4000 && call.input.operation.at_ms < 8000)));
  const boundary = await browser.$$(".director-segment-boundary")[0];
  await boundary.waitForEnabled();
  const boundaryCenter = await center(boundary);
  const targetCenter = await center(await browser.$$(".director-point")[2]);
  await drag(boundaryCenter, { x: targetCenter.x, y: boundaryCenter.y });
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.kind === "move_segment_boundary" && call.input.operation.point_id === "p2")));
  assert.equal((await browser.$$(".director-point")).length, 7);
  assert.equal((await browser.$$(".director-segment")).length, 3);
  await browser.execute(() => window.holdDirectorEdit());
  const completedEdits = await browser.execute(() => window.directorEditFinished ?? 0);
  await browser.$('[aria-label="调整整体时长"]').click();
  await browser.$('[role="dialog"]').waitForDisplayed();
  await setInput('[aria-label^="整体时长"]', "36");
  await browser.$('[aria-label^="整体时长"]').click();
  await browser.keys("Tab");
  await browser.waitUntil(async () => browser.execute(() => window.directorCommands.some((call) => call.command === "edit_chapter_visual_plan_draft" && call.input.operation.duration_ms === 36000)));
  await browser.execute(() => window.switchDirectorChapter(2));
  await browser.waitUntil(async () => (await browser.$(".director-header h2").getText()) === "第二章");
  assert.ok(await browser.$('[aria-label="保存草稿"]').isEnabled(), "a pending edit in the previous chapter must not lock this chapter");
  await browser.execute(() => window.releaseDirectorEdit());
  await browser.waitUntil(async () => browser.execute((count) => window.directorEditFinished > count, completedEdits));
  await browser.execute(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.match(await browser.$('[aria-label="调整整体时长"]').getText(), /12s/);
  assert.doesNotMatch(await browser.$(".director-save-status").getText(), /保存中/);
  await browser.execute(() => window.switchDirectorChapter(3));
  await browser.$(".director-empty").waitForDisplayed();
  await browser.$("button=重试加载").waitForDisplayed();
  assert.equal(await browser.$(".director-viewer").isExisting(), false, "a failed chapter load must never display the previous chapter's plan");
  const report = { passed: true, cases: ["all four frame aspect ratios", "viewport fitting inside the real chapter workspace", "compact timeline reserves preview space", "shared track hit targets", "timing dialog and cancelled edits", "default desktop inspector", "container resize without viewport resize", "read-only controls", "points independent of segments", "play/pause", "prompt autosave payload", "proportional retime request", "responsive overflow", "mobile dialog and Escape", "mobile media, size and candidate actions clear of footer", "mobile bottom actions reachable", "dialog keyboard isolation", "point and segment boundary pointer gestures", "chapter switch during pending mutation", "chapter load failure never displays the previous plan"], measurements };
  report.nativeEditingChecks = ["one edit settles after a reordered JSON acknowledgement without repeated saves or confirmation flicker", "late save preserves newer manual input", "SSE receipts apply without editor reload or remount", "selection, playhead and focus survive receipt application", "batch undo preserves intervening manual edits", "overlapping changes pause autosave and preserve both versions", "conflict resolution merges unaffected fields"];
  await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
  console.log(JSON.stringify(report));
} catch (error) {
  if (browser) {
    await browser.saveScreenshot(path.join(artifacts, "failure.png"));
    await writeFile(path.join(artifacts, "failure.html"), await browser.getPageSource());
  }
  throw error;
} finally {
  if (browser) await browser.deleteSession();
  for (const child of processes) if (child.exitCode == null) child.kill("SIGTERM");
  await writeFile(path.join(artifacts, "process.log"), logs.join(""));
}
