import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { buildPageFixtures, seedLocalModels } from "./fixtures.mjs";
import { directorSnapshot, homeAgentFixtures, videoDirectorSnapshot } from "./home-agent-fixtures.mjs";

const seed = JSON.parse(await readFile(new URL("./novel-fixtures.json", import.meta.url)));
const runtimeSeed = JSON.parse(await readFile(new URL("./novel-runtime.json", import.meta.url)));
const mediaSeed = JSON.parse(await readFile(new URL("./media-fixtures.json", import.meta.url)));
seed.commands.get_project.content_revision ??= 0;
for (const project of seed.projectRecords) project.content_revision ??= 0;
const directorPlan = mediaSeed.byChapter[1].get_chapter_visual_plan.draft.plan;
const output = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/novel/", import.meta.url));
const downloadOutput = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/novel-downloads/", import.meta.url));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";
assert.match(phase, /^[a-z0-9_-]+$/i);
const widths = process.env.A4N_UI_WIDTHS
  ? process.env.A4N_UI_WIDTHS.split(",").map(Number)
  : [1440, 900, 390];
const height = 1000;
const timeout = 5000;
const report = [];
const pageSelector = ".a4n-page-container";
const rootTitle = seed.commands.get_project.title;
let current = null;
let auditWindow = null;
let navigation = 0;
let nativeDraftStorageScenario = false;

const literal = (value) => {
  if (!value.includes("'")) return "'" + value + "'";
  if (!value.includes('"')) return '"' + value + '"';
  return "concat(" + value.split("'").map((part) => "'" + part + "'").join(',"\'",') + ")";
};
const textButton = (label, scope) => {
  const query = (scope ? ".//" : "//") + "button[normalize-space(.)=" + literal(label) + "]";
  return scope ? scope.$(query) : $(query);
};
const ariaButton = (label, scope) => {
  const query = "button[aria-label=" + JSON.stringify(label) + "]";
  return scope ? scope.$(query) : $(query);
};
const containing = (text, scope, tags = "button") => {
  const query = (scope ? ".//" : "//") + "*[(" + tags.split(",").map((tag) => "self::" + tag).join(" or ") + ") and contains(normalize-space(.)," + literal(text) + ")]";
  return scope ? scope.$(query) : $(query);
};
async function press(element) {
  await element.waitForDisplayed({ timeout });
  const reachable = await wheelTo(element);
  assert.equal(reachable.reached, true, "Control is not pointer-reachable: " + reachable.label);
  assert.equal(reachable.enabled, true, "Control is disabled: " + reachable.label);
  await element.click();
  return element;
}
async function clearByKeyboard(element) {
  await element.click();
  await browser.keys(["CTRL", "A"]);
  await browser.keys("BACKSPACE");
}
async function clickText(label, scope) {
  return press(await textButton(label, scope));
}
async function menuItem(label) {
  const item = await $('//*[@role="menuitem" and contains(normalize-space(.),' + literal(label) + ')]');
  return press(item);
}
async function dialog(title) {
  let selected;
  await browser.waitUntil(async () => {
    const dialogs = [...await $$('[role="dialog"]')].reverse();
    for (const item of dialogs) {
      if (!await item.isDisplayed()) continue;
      const state = await item.execute((node) => ({
        exiting: node.closest(".a4n-modal-layer")?.getAttribute("data-modal-state") === "exiting",
        label: (node.getAttribute("aria-labelledby") ?? "").split(/\s+/).filter(Boolean)
          .map((id) => node.ownerDocument.getElementById(id)?.textContent ?? "").join(" ").trim()
          || node.getAttribute("aria-label"),
      }));
      if (state.exiting || (title && state.label !== title)) continue;
      selected = item;
      return true;
    }
    return false;
  }, { timeout, timeoutMsg: "No visible dialog: " + (title ?? "topmost") });
  return selected;
}
async function waitText(scope, text) {
  await browser.waitUntil(async () => (await scope.getText()).includes(text), {
    timeout, timeoutMsg: "Expected populated content: " + text,
  });
}
async function waitForAgentMessage(text, waitTimeout = timeout) {
  try {
    await browser.waitUntil(async () => browser.execute((expected) =>
      [...document.querySelectorAll(".agent-message-list")]
        .some((list) => list.innerText.includes(expected)),
    text), { timeout: waitTimeout, timeoutMsg: "Expected a visible Agent message: " + text });
  } catch (error) {
    current.evidence = {
      ...current.evidence,
      finalConsole: await browser.execute(() => {
        const root = document.querySelector('[aria-label="领域 Agent 控制台"]');
        return root ? {
          text: root.innerText,
          threadStatus: root.querySelector(".agent-console-thread-status")?.textContent ?? null,
          lists: [...root.querySelectorAll(".agent-message-list")].map((list) => list.innerText),
        } : null;
      }),
    };
    throw error;
  }
}
async function respondToPending(command, body, status = 200) {
  await browser.execute(({ command, body, status }) => {
    const audit = window.__uiAudit;
    const index = audit.pending.findIndex((item) => item.command === command);
    if (index < 0) throw new Error("No pending fixture request: " + command);
    const [request] = audit.pending.splice(index, 1);
    audit.deferred = audit.deferred.filter((item) => item !== command);
    request.resolve(Response.json(body, { status }));
  }, { command, body, status });
}
async function openProjectAction(label) {
  await project();
  await press(await $('button[title="更多项目操作"]'));
  await press(await containing(label, await dialog()));
  return dialog(label);
}
async function characterTab(label, section) {
  const scope = await project("角色");
  await press(await scope.$('button[aria-label="林岚 · 查看角色详情"]'));
  const detail = await dialog();
  await clickText(label, detail);
  if (section) await clickText(section, detail);
  return detail;
}
async function portraits() {
  const scope = await project("角色");
  await clickText("立绘", scope);
  await scope.$("img").waitForDisplayed({ timeout });
  return scope;
}
async function relationshipGraph(scope, phase) {
  let graph;
  await browser.waitUntil(async () => {
    graph = await scope.execute((root) => ({
      nodes: root.querySelectorAll(".react-flow__node").length,
      edgeLengths: [...root.querySelectorAll(".react-flow__edge-path")].map((edge) => edge.getTotalLength()),
    }));
    current.evidence = { ...current.evidence, [phase]: graph };
    return graph.nodes === 12 && graph.edgeLengths.length === 26 && graph.edgeLengths.every((length) => length > 0);
  }, { timeout, timeoutMsg: "Relationship nodes/edges are incomplete after " + phase });
  return graph;
}
async function inspiration() {
  await home();
  await press(await (await rootRow()).$('button[aria-label="更多操作"]'));
  await menuItem("灵感对话");
  await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/inspiration/1", { timeout });
  await $('textarea[aria-label="输入灵感对话内容"]').waitForDisplayed({ timeout });
  await browser.waitUntil(async () => (await $$(".inspiration-option-card")).length === 3, { timeout });
  return $("main");
}
async function openHeaderMenu() {
  for (const button of await $$('.a4n-app-page-header button[aria-label="更多操作"]')) {
    if (await button.isDisplayed()) return press(button);
  }
  throw new Error("No visible header menu");
}
async function blueprintConfirmation() {
  const scope = await inspiration();
  await clickText("生成首版蓝图", scope);
  const progress = await dialog("生成首版蓝图");
  await (await textButton("查看蓝图", progress)).waitForDisplayed({ timeout });
  await clickText("查看蓝图", progress);
  await browser.waitUntil(async () => !await progress.isDisplayed(), { timeout });
  await waitText(scope, "确认进入项目");
  return scope;
}
async function blueprintRefinement() {
  const scope = await blueprintConfirmation();
  await clickText("调整蓝图", scope);
  const refine = await dialog("调整蓝图");
  await refine.$("textarea").setValue(seed.commands.preview_refine_blueprint_request.steps[0].user_prompt);
  return refine;
}
async function checkRequestPreview(scope, command) {
  const step = seed.commands[command].steps[0];
  const preview = seed.commands[command];
  if (preview.steps.length === 1 && step.title === preview.title) {
    assert.equal(await scope.$("h2").getText(), "请求信息");
    const presentation = await scope.execute((node) => {
      const templateRow = [...node.querySelectorAll("dl > div")].find((item) => item.querySelector("dt")?.textContent?.trim() === "模板");
      return {
        dialogOpacity: getComputedStyle(node).getPropertyValue("--a4n-dialog-opacity").trim(),
        template: templateRow?.querySelector("dd")?.textContent?.trim() ?? null,
      };
    });
    assert.equal(presentation.dialogOpacity, "1", "Request preview content must use an opaque surface");
    if (step.prompt_name) assert.equal(presentation.template, step.prompt_name);
  }
  const messageViewport = await scope.execute((node) => {
    const body = node.querySelector('[data-modal-body="1"]');
    const lastMessage = node.querySelector("#request-preview-step-0 > section > div.grid > section:last-child");
    if (!body || !lastMessage) return null;
    const startScrollTop = body.scrollTop;
    const bodyRect = body.getBoundingClientRect();
    const messageRect = lastMessage.getBoundingClientRect();
    const targetScrollTop = Math.min(
      body.scrollHeight - body.clientHeight,
      startScrollTop + Math.max(0, messageRect.bottom - bodyRect.bottom + 16),
    );
    body.scrollTop = targetScrollTop;
    const visibleRect = lastMessage.getBoundingClientRect();
    const gap = body.getBoundingClientRect().bottom - visibleRect.bottom;
    const visible = visibleRect.top >= bodyRect.top - 1 && gap >= 15;
    body.scrollTop = startScrollTop;
    return { visible, gap, scrollRange: body.scrollHeight - body.clientHeight };
  });
  if (step.messages.length > 0) assert.ok(messageViewport?.visible, "The last prompt message must scroll fully above the fixed footer");
  for (const message of step.messages) {
    const prompt = await scope.$(".//pre[normalize-space(.)=" + literal(message.content) + "]");
    await prompt.waitForDisplayed({ timeout });
    assert.equal(await prompt.getText(), message.content);
  }
  const request = await browser.execute((name) => window.__uiAudit.requests.findLast((item) => item.command === name)?.body.input, command);
  assert.equal(request?.project_id, 1, "The preview request must target this project");
  if (command === "preview_inspiration_request") assert.equal(request.message, step.user_prompt);
  if (command === "preview_refine_blueprint_request") assert.equal(request.focus, step.user_prompt);
  if (command === "preview_generate_blueprint_request") {
    assert.equal(request.target_length, 9);
    assert.ok(request.variables.inspiration_transcript.includes(seed.commands.get_inspiration_history[0].content));
  }
  return { previewCommand: command, promptMessages: step.messages.length, promptCharacters: step.total_chars, messageViewport };
}
async function lastRequestPreview(scope) {
  await press(await containing("完整请求", scope, "summary"));
  await press(await containing("上下文详情", scope, "summary"));
  return (await scope.$$("pre")).at(-1);
}
async function lightbox() {
  const scope = await $('.a4n-modal-layer[data-modal-state="entering"] .a4n-image-lightbox-panel[role="dialog"]');
  await scope.waitForDisplayed({ timeout });
  const image = await scope.$(".a4n-image-lightbox-image");
  await browser.waitUntil(async () => image.execute((node) => node.complete && node.naturalWidth > 0), { timeout });
  return scope;
}
async function assertFocus(element) {
  await browser.waitUntil(() => element.isFocused(), { timeout, timeoutMsg: "Escape did not return focus to the opening control" });
}
async function previewRoundTrip(trigger) {
  await press(trigger);
  await lightbox();
  await closeDialog();
  await assertFocus(trigger);
  await press(trigger);
  return lightbox();
}
async function writingPanel(label) {
  await writing();
  await press(await $('.a4n-writing-panel-toggle button[title^=' + JSON.stringify(label) + ']'));
  const panel = await $('aside[aria-label=' + JSON.stringify(label) + ']');
  await panel.waitForDisplayed({ timeout });
  return panel;
}
async function waitForOptimizerChapter(title) {
  await browser.waitUntil(async () => browser.execute((expected) =>
    document.querySelector('aside[aria-label="写作助手"] section > header')?.textContent.includes(expected) ?? false,
  title), { timeout, timeoutMsg: `The optimizer header must identify ${title}` });
}
async function directorInspector(kind = "point") {
  await videoDirector();
  await press(await $(`.director-${kind}`));
  return selectedDirectorInspector(kind);
}
async function selectedDirectorInspector(kind) {
  const aside = await $('aside[aria-label="所选对象属性"]');
  return await aside.isDisplayed() ? aside : dialog(kind === "point" ? "图片点属性" : "镜头段属性");
}
async function openScriptScene(scope, index) {
  await scope.$('details[class~="group/scene"]').waitForDisplayed({ timeout });
  const scenes = await scope.$$('details[class~="group/scene"]');
  const scene = scenes.at(index);
  const heading = await scene.$(":scope > summary");
  if (await scene.getAttribute("open") == null) await press(heading);
  for (const summary of await scene.$$("summary")) {
    if ((await summary.getText()).startsWith("镜头 ") && await summary.execute((node) => !node.parentElement.open)) await press(summary);
  }
  await reach(heading);
  return scene;
}
async function home() {
  auditWindow ??= await browser.getWindowHandle();
  await browser.switchToWindow(auditWindow);
  // A distinct document prevents a prior modal/draft/BiDi context leaking into the next case.
  await browser.url("/?ui-page-reset=" + (++navigation) + (nativeDraftStorageScenario ? "&ui-native-draft-storage=1" : ""));
  await $(".a4n-home-content").waitForDisplayed({ timeout });
  return $(".a4n-home-content");
}
async function switchInterfaceLanguage(option, expectedLocale, triggerLabel) {
  const statusPanel = $("#a4n-status-bar-panel");
  if (!await statusPanel.isDisplayed().catch(() => false)) {
    await browser.execute(() => document.querySelector(".a4n-status-bar-orb")?.click());
    await statusPanel.waitForDisplayed({ timeout });
  }
  await browser.execute((label) => [...document.querySelectorAll("#a4n-status-bar-panel button")]
    .find((button) => button.getAttribute("aria-label") === label)?.click(), triggerLabel);
  await $("[role=menu]").waitForDisplayed({ timeout });
  await menuItem(option);
  await browser.waitUntil(async () => browser.execute((locale) => document.documentElement.lang === locale, expectedLocale), { timeout });
  await browser.execute(() => document.querySelector("#a4n-status-bar-panel")?.focus());
  await browser.keys("Escape");
  await statusPanel.waitForExist({ reverse: true, timeout });
}
async function rootRow() {
  return $('//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=' + literal(rootTitle) + ']]');
}
async function project(tab = "概览", beforeTab, beforeProject) {
  await home();
  const row = await rootRow();
  await press(await row.$('button[aria-label="更多操作"]'));
  if (beforeProject) await beforeProject();
  await menuItem("查看档案");
  await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/novel/1", { timeout });
  await $(pageSelector).waitForDisplayed({ timeout });
  if (beforeTab) await beforeTab();
  const rail = await $("nav[role=tablist]");
  await clickText(tab, rail);
  return $(pageSelector);
}
async function collaborationPanel(beforeProject) {
  const overview = await project("概览", undefined, beforeProject);
  await press(await containing("协作成员", overview));
  const panel = await dialog("协作成员");
  await waitText(panel, "共 3 位成员");
  return panel;
}
async function openImportReviewEditor(aside) {
  const details = await aside.$("details");
  if (!(await details.getProperty("open"))) await press(await details.$("summary"));
  const editor = await details.$("textarea");
  await editor.waitForDisplayed({ timeout });
  await editor.scrollIntoView({ block: "center" });
  return editor;
}
async function assertProjectTitleVisible() {
  const state = await browser.execute(() => {
    const title = document.querySelector(".a4n-app-page-header--project-detail h1");
    return title ? {
      text: title.textContent?.trim() ?? "",
      width: title.clientWidth,
      scrollWidth: title.scrollWidth,
      height: title.clientHeight,
      scrollHeight: title.scrollHeight,
    } : null;
  });
  assert.ok(state?.text, "The project title must be present");
  assert.ok(state.width > 0, "The project title must have a visible box");
  assert.ok(state.scrollWidth <= state.width + 1, "The project title must not be horizontally clipped");
  assert.ok(state.scrollHeight <= state.height + 1, "The project title must not be vertically clipped");
  return { projectTitle: state.text, titleFullyVisible: true };
}
async function writing(tab = "正文") {
  await home();
  const row = await rootRow();
  await press(await row.$('button[aria-label$=" · 进入写作桌"]'));
  await $(".a4n-writing-panel-toggle").waitForDisplayed({ timeout });
  const activePanel = await $('.a4n-writing-panel-toggle button[aria-pressed="true"]');
  if (await activePanel.isExisting()) {
    if (await $('[role="dialog"]').isDisplayed()) await closeDialog();
    else await press(activePanel);
  }
  await $(".a4n-writing-workspace").waitForDisplayed({ timeout });
  // The workspace deliberately remembers its last tab across page visits.
  await clickText("正文", await $(".a4n-writing-tabs"));
  const editor = await $('textarea[aria-label="章节正文"]');
  await editor.waitForDisplayed({ timeout });
  // The fixture has three chapters. Use the real previous-chapter control.
  for (let index = 0; index < 3; index++) {
    const previous = await ariaButton("上一章");
    if (!await previous.isEnabled()) break;
    const before = await editor.getValue();
    await press(previous);
    await browser.waitUntil(async () => (await editor.getValue()) !== before, { timeout });
  }
  assert.equal(await editor.getValue(), seed.commands.list_chapters[0].content);
  if (tab !== "正文") await clickText(tab, await $(".a4n-writing-tabs"));
  return $(".a4n-writing-workspace");
}
async function videoProjectStage(stage, beforeSelect) {
  await browser.url("/video-project/3");
  const scope = await $(pageSelector);
  await scope.waitForDisplayed({ timeout });
  await beforeSelect?.();
  const selector = await $('[role="combobox"][id="video-stages-3-select"]');
  if (await selector.isDisplayed()) {
    await selector.click();
    const option = await $(`//*[@role="option" and contains(normalize-space(.), ${literal(stage)})]`);
    await option.waitForClickable();
    await option.click();
  } else {
    await press(await $(`//*[@role="tab" and contains(normalize-space(.), ${literal(stage)})]`));
  }
  if (stage === "导演编排") await scope.$(".director-timeline").waitForDisplayed({ timeout });
  else await browser.waitUntil(async () => (await scope.getText()).includes("场景 1"), { timeout });
  return scope;
}
const videoScreenplay = () => videoProjectStage("拍摄剧本");
const videoDirector = (beforeSelect) => videoProjectStage("导演编排", beforeSelect);
async function closeDialog() {
  const closing = await dialog();
  await browser.keys("Escape");
  await browser.waitUntil(async () => !await closing.isDisplayed(), { timeout, timeoutMsg: "Escape did not close the active dialog" });
}
async function closeDomainAgentDialog() {
  const closing = await dialog();
  await press(await ariaButton("关闭"));
  await browser.waitUntil(async () => !await closing.isDisplayed(), { timeout, timeoutMsg: "The Agent close control did not close the active dialog" });
}
async function wheelTo(element, anchor = "center") {
  await element.waitForExist({ timeout });
  const inspect = () => element.execute((node, anchor) => {
    const r = node.getBoundingClientRect();
    const clip = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
    for (let parent = node.parentElement; parent; parent = parent.parentElement) {
      const style = getComputedStyle(parent);
      const bounds = parent.getBoundingClientRect();
      if (/(auto|scroll|hidden|clip)/.test(style.overflowX)) {
        clip.left = Math.max(clip.left, bounds.left); clip.right = Math.min(clip.right, bounds.right);
      }
      if (/(auto|scroll|hidden|clip)/.test(style.overflowY)) {
        clip.top = Math.max(clip.top, bounds.top); clip.bottom = Math.min(clip.bottom, bounds.bottom);
      }
    }
    const x = anchor === "end" ? r.right - Math.min(6, r.width / 2) : r.left + r.width / 2;
    const y = anchor === "start" ? r.top + 1 : anchor === "end" ? r.bottom - Math.min(6, r.height / 2) : r.top + r.height / 2;
    const inViewport = x > clip.left && x < clip.right && y > clip.top && y < clip.bottom;
    const hit = inViewport ? document.elementFromPoint(x, y) : null;
    const hitTarget = Boolean(hit && (node === hit || node.contains(hit)));
    const contentEndPending = anchor === "end" && hitTarget && /(auto|scroll)/.test(getComputedStyle(node).overflowY)
      && node.scrollHeight - node.clientHeight - node.scrollTop > 1;
    const reached = hitTarget && !contentEndPending;
    const axis = contentEndPending ? "y" : x <= clip.left || x >= clip.right ? "x" : y <= clip.top || y >= clip.bottom ? "y" : null;
    let scroll = contentEndPending ? node : node.parentElement;
    while (scroll && axis) {
      const style = getComputedStyle(scroll), box = scroll.getBoundingClientRect();
      const overflow = axis === "x" ? style.overflowX : style.overflowY;
      const extent = axis === "x" ? scroll.scrollWidth - scroll.clientWidth : scroll.scrollHeight - scroll.clientHeight;
      const visibleWidth = Math.min(innerWidth, box.right) - Math.max(0, box.left);
      const visibleHeight = Math.min(innerHeight, box.bottom) - Math.max(0, box.top);
      if (/(auto|scroll)/.test(overflow) && extent > 1 && visibleWidth > 16 && visibleHeight > 16) break;
      scroll = scroll.parentElement;
    }
    const surface = scroll && axis ? scroll.getBoundingClientRect() : null;
    const bounds = surface ? {
      left: Math.max(1, surface.left), right: Math.min(innerWidth - 1, surface.right),
      top: Math.max(1, surface.top), bottom: Math.min(innerHeight - 1, surface.bottom),
    } : null;
    const origin = bounds ? { x: (bounds.left + bounds.right) / 2, y: (bounds.top + bounds.bottom) / 2 } : null;
    const span = bounds ? axis === "x" ? bounds.right - bounds.left : bounds.bottom - bounds.top : 0;
    const distance = contentEndPending ? node.scrollHeight - node.clientHeight - node.scrollTop
      : origin ? axis === "x" ? x - origin.x : y - origin.y : 0;
    return {
      label: node.getAttribute("aria-label") || node.getAttribute("title") || node.innerText?.trim().slice(0, 100),
      rect: { x: r.x, y: r.y, width: r.width, height: r.height, bottom: r.bottom, right: r.right },
      inViewport, reached, clip, axis, scrollable: Boolean(surface), origin,
      delta: Math.round(Math.max(-span, Math.min(span, distance))),
      enabled: !node.matches(":disabled") && node.getAttribute("aria-disabled") !== "true",
    };
  }, anchor);
  let state = await inspect();
  let count = 0;
  let wheelSteps = 0;
  for (; count < 24 && !state.reached; count++) {
    if (!state.axis) {
      await browser.pause(120);
    } else if (state.scrollable && state.delta) {
      await browser.performActions([{ type: "wheel", id: "novel-wheel", actions: [{
        type: "scroll", origin: "viewport",
        x: Math.round(state.origin.x), y: Math.round(state.origin.y),
        deltaX: state.axis === "x" ? state.delta : 0,
        deltaY: state.axis === "y" ? state.delta : 0, duration: 120,
      }] }]);
      wheelSteps++;
      await browser.releaseActions();
      await browser.pause(100);
    } else {
      break;
    }
    state = await inspect();
  }
  return { ...state, wheelSteps, method: wheelSteps ? "native wheel" : "already visible" };
}
async function reach(element) {
  return wheelTo(element, "end");
}
async function snapshot(id, width, stage, scope, evidence = {}) {
  await browser.switchToWindow(auditWindow);
  await browser.pause(220); // CSS transitions only; the particle logo never finishes animating.
  const state = await browser.execute(() => ({
    width: innerWidth, height: innerHeight, devicePixelRatio, visualScale: visualViewport?.scale ?? null,
    url: location.pathname + location.search,
    overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    theme: document.documentElement.classList.contains("dark") ? "dark" : "light",
    unexpected: [...window.__uiAudit.unexpected], errors: [...window.__uiAudit.errors],
    commands: [...new Set(window.__uiAudit.commands)],
  }));
  const text = await scope.getText();
  const file = phase + "-" + id + "-" + width + "-" + stage + ".png";
  await browser.saveScreenshot(output + file);
  const png = await readFile(output + file);
  const findings = [];
  const capturedUrl = new URL(await browser.getUrl());
  if (capturedUrl.pathname + capturedUrl.search !== state.url) findings.push("navigation changed during capture");
  if (state.width !== width || png.readUInt32BE(16) !== width) findings.push("viewport mismatch");
  if (state.overflow !== 0) findings.push("horizontal overflow: " + state.overflow);
  if (state.unexpected.length) findings.push("missing fixture: " + state.unexpected.join(", "));
  if (state.errors.length) findings.push("browser errors: " + state.errors.join(", "));
  report.push({ id, stage, file, task: current.task, ...state, evidence, findings, text: text.slice(0, 12000), textLength: text.length });
  return findings;
}
async function matrix(spec, selectedWidths = widths) {
  current = { id: spec.id, task: spec.task, visited: [], plannedWidths: selectedWidths };
  const failures = [];
  try {
    for (const width of selectedWidths) {
      current.width = width;
      current.evidence = undefined;
      await browser.setWindowSize(width, height);
      const scope = await spec.open(width);
      for (const text of spec.text ?? []) await waitText(scope, text);
      if (spec.captureInitialBeforeCheck) {
        failures.push(...await snapshot(spec.id, width, "initial", scope));
      }
      const result = spec.check ? await spec.check(scope, width) : {};
      const captureScope = result?.scope ?? scope;
      const evidence = Object.fromEntries(Object.entries(result ?? {}).filter(([key]) => key !== "scope"));
      failures.push(...await snapshot(spec.id, width, spec.captureInitialBeforeCheck ? "verified" : "initial", captureScope, evidence));
      if (spec.last) {
        const target = await spec.last(captureScope);
        const last = await reach(target);
        failures.push(...await snapshot(spec.id, width, "last", captureScope, { last }));
        if (!last.reached) failures.push(width + ": last item is obscured or cannot be user-scrolled: " + last.label);
      }
      if (spec.cleanup) current.cleanup = await spec.cleanup(captureScope, width);
      current.visited.push(width);
    }
    assert.deepEqual(failures, [], spec.id + " measured findings");
  } catch (error) {
    current.error = error.stack ?? error.message;
    throw error;
  }
}
async function saveReport() {
  await writeFile(output + phase + "-report.json", JSON.stringify(report, null, 2));
}

before(async function () {
  this.timeout(60000);
  await mkdir(output, { recursive: true });
  const fixtures = await buildPageFixtures(browser.options.baseUrl);
  await installUiFixtures(browser, fixtures);
  await browser.addInitScript(() => {
    window.addEventListener("error", (event) => {
      (window.__uiAudit.errorDetails ??= []).push({
        message: event.message, stack: event.error?.stack ?? null,
        filename: event.filename, line: event.lineno, column: event.colno,
      });
    });
    window.addEventListener("unhandledrejection", (event) => {
      (window.__uiAudit.errorDetails ??= []).push({ message: String(event.reason), stack: event.reason?.stack ?? null });
    });
  });
  await home();
  const activeTasks = await browser.execute(async () => {
    const { generationClient } = await import("/src/api/generationTasks.ts");
    return generationClient.listActive(1);
  });
  assert.deepEqual(activeTasks, [], "The completed blueprint fixture must not appear as an active task");
  const models = await seedLocalModels(browser);
  await browser.execute(async (id) => {
    const local = await import("/src/api/localRuntimeSettings.ts");
    await local.setDefaultLocalImageConfig(id);
  }, models.image);
});
after(saveReport);
afterEach(async function () {
  if (this.currentTest.state === "failed") {
    const state = await browser.execute(() => ({
      url: location.href, width: innerWidth, text: document.body.innerText.slice(0, 12000),
      readyState: document.readyState, rootMarkup: document.querySelector("#root")?.innerHTML.slice(0, 2400) ?? null,
      scriptUrls: [...document.scripts].map((script) => script.src).filter(Boolean),
      requests: window.__uiAudit?.requests?.slice(-24) ?? [],
      commands: window.__uiAudit?.commands ?? [],
      unexpected: window.__uiAudit?.unexpected, errors: window.__uiAudit?.errors,
      workspaceTab: document.querySelector('.a4n-writing-tabs [aria-selected="true"]')?.textContent?.trim(),
      errorDetails: window.__uiAudit?.errorDetails ?? [],
      dependencyUrls: performance.getEntriesByType("resource").map((entry) => entry.name).filter((url) => /react|xyflow/.test(url)),
    }));
    const browserLogs = await browser.getLogs("browser").catch((error) => [{ level: "error", message: String(error) }]);
    const file = phase + "-" + (current?.id ?? "setup") + "-" + state.width + "-failure.png";
    await browser.saveScreenshot(output + file);
    const failure = {
      id: current?.id, file, outcome: "failed", ...state,
      browserLogs,
      error: current?.error ?? this.currentTest.err?.message,
      evidence: current?.evidence,
      notCapturedWidths: (current?.plannedWidths ?? widths).filter((width) => !current?.visited?.includes(width)),
    };
    report.push(failure);
    await writeFile(output + file.replace(/\.png$/, ".json"), JSON.stringify(failure, null, 2));
  }
  await saveReport();
});

describe("Novel page audit - ordinary populated pages", function () {
  this.timeout(60000);

  it("UX-1861 keeps collaboration writes visible when directory reads fail", async function () {
    current = { id: "UX-1861-collaboration-writes", task: "协作者成功写入后，即使列表刷新失败也立即显示已提交状态", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await collaborationPanel();
    const requestCount = (command) => browser.execute((name) =>
      window.__uiAudit.requests.filter((item) => item.command === name).length, command);
    const identity = await panel.$('input[aria-label="用户名或邮箱"]');
    const inviteRole = await panel.$('button[aria-label="邀请角色"]');
    assert.equal(await identity.getAttribute("aria-label"), "用户名或邮箱");
    assert.equal(await inviteRole.getAttribute("aria-label"), "邀请角色");

    const createdInvite = {
      id: 103,
      project_title: rootTitle,
      inviter_username: "User1",
      invitee_username: "新邀请用户",
      role: "editor",
    };
    const inviteCount = await requestCount("invite_project_member");
    await browser.execute((invite) => {
      const audit = window.__uiAudit;
      audit.fixtures.invite_project_member = invite;
      audit.fixtures.list_project_invites.unshift(invite);
      audit.failNext.push({ command: "list_project_invites", status: 503, message: "邀请目录读取失败" });
    }, createdInvite);
    await identity.setValue("new.user@example.test");
    await browser.keys("Enter");
    await browser.waitUntil(async () => (await requestCount("invite_project_member")) === inviteCount + 1, { timeout });
    await waitText(panel, "新邀请用户");
    await waitText($("body"), "邀请已发送");
    await waitText(panel, "邀请目录读取失败");

    const updatedMembers = await browser.execute(() => {
      const audit = window.__uiAudit;
      return structuredClone(audit.fixtures.list_project_members.map((member) =>
        member.user_id === 2 ? { ...member, role: "viewer" } : member,
      ));
    });
    const roleCount = await requestCount("update_project_member_role");
    const memberReadsBeforeRole = await requestCount("list_project_members");
    await browser.execute((members) => {
      const audit = window.__uiAudit;
      audit.fixtures.update_project_member_role = null;
      audit.fixtures.list_project_members = members;
    }, updatedMembers);
    const editorRoleSelector = 'button[aria-label="调整 协作编辑 · 长文本核对 的角色"]';
    const editorRole = await panel.$(editorRoleSelector);
    await press(editorRole);
    await browser.waitUntil(async () =>
      await panel.$(editorRoleSelector).getAttribute("aria-expanded") === "true",
    { timeout, timeoutMsg: "成员角色选择菜单未打开" });
    await press(await $('//*[@role="option" and normalize-space(.)="只读成员"]'));
    await browser.waitUntil(async () => (await requestCount("update_project_member_role")) === roleCount + 1, { timeout });
    await waitText($("body"), "成员角色已更新");
    await browser.waitUntil(() => editorRole.isFocused(), { timeout, timeoutMsg: "Role update must retain keyboard focus on the member control" });
    assert.ok((await editorRole.getText()).includes("只读成员"));
    assert.equal(await requestCount("list_project_members"), memberReadsBeforeRole,
      "A confirmed member-role write updates its local row without a redundant directory read");

    const memberRow = await panel.$('.//li[.//div[contains(normalize-space(.),"协作编辑 · 长文本核对")]]');
    const removeCount = await requestCount("remove_project_member");
    const memberReadsBeforeRemove = await requestCount("list_project_members");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.fixtures.remove_project_member = null;
      audit.fixtures.list_project_members = audit.fixtures.list_project_members.filter((member) => member.user_id !== 2);
    });
    await press(await memberRow.$('button[aria-label="更多操作"]'));
    await menuItem("移除成员");
    const removeConfirm = await dialog("移除协作者");
    await clickText("移除", removeConfirm);
    await browser.waitUntil(async () => (await requestCount("remove_project_member")) === removeCount + 1, { timeout });
    await browser.waitUntil(async () => !(await memberRow.isExisting()), { timeout });
    await waitText($("body"), "协作者已移除");
    assert.equal(await requestCount("list_project_members"), memberReadsBeforeRemove,
      "A confirmed member removal updates its local list without a redundant directory read");

    const inviteRow = await panel.$('.//li[.//span[normalize-space(.)="新邀请用户"]]');
    const revokeCount = await requestCount("revoke_project_invite");
    const inviteReadsBeforeRevoke = await requestCount("list_project_invites");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.fixtures.revoke_project_invite = null;
      audit.fixtures.list_project_invites = audit.fixtures.list_project_invites.filter((invite) => invite.id !== 103);
    });
    await press(await inviteRow.$('button[aria-label="更多操作"]'));
    await menuItem("撤销邀请");
    await browser.waitUntil(async () => (await requestCount("revoke_project_invite")) === revokeCount + 1, { timeout });
    await browser.waitUntil(async () => !(await inviteRow.isExisting()), { timeout });
    await waitText($("body"), "邀请已撤销");
    assert.equal(await requestCount("list_project_invites"), inviteReadsBeforeRevoke,
      "A confirmed invite revocation updates its local list without a redundant directory read");
    assert.deepEqual(await snapshot(current.id, 1440, "projected-writes", panel, {
      inviteCreated: true, roleUpdated: true, memberRemoved: true, inviteRevoked: true,
      readFailures: ["invite list"],
    }), []);
    current.visited.push(1440);
  });

  it("UX-1865 keeps concurrent member writes visible while parent refreshes settle", async function () {
    current = { id: "UX-1865-collaboration-concurrent-refresh", task: "并发修改不同成员后保留本地写入结果并忽略过期项目刷新", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await collaborationPanel();
    const requestCount = (command) => browser.execute((name) =>
      window.__uiAudit.requests.filter((item) => item.command === name).length, command);
    const roleCount = await requestCount("update_project_member_role");
    const projectReadCount = await requestCount("get_project");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.fixtures.update_project_member_role = null;
      audit.deferred.push("get_project");
    });

    const firstRoleSelector = 'button[aria-label="调整 协作编辑 · 长文本核对 的角色"]';
    const firstRole = await panel.$(firstRoleSelector);
    await press(firstRole);
    await browser.waitUntil(async () =>
      await panel.$(firstRoleSelector).getAttribute("aria-expanded") === "true",
    { timeout, timeoutMsg: "第一个成员角色选择菜单未打开" });
    await press(await $('//*[@role="option" and normalize-space(.)="只读成员"]'));
    await browser.waitUntil(async () => (await requestCount("update_project_member_role")) === roleCount + 1, { timeout });
    await browser.waitUntil(async () => (await panel.$(firstRoleSelector).getText()).includes("只读成员"), { timeout });
    await browser.waitUntil(async () => (await requestCount("get_project")) === projectReadCount + 1, { timeout });
    const secondRoleSelector = 'button[aria-label="调整 只读审阅者 的角色"]';
    const secondRole = await panel.$(secondRoleSelector);
    await press(secondRole);
    await browser.waitUntil(async () =>
      await panel.$(secondRoleSelector).getAttribute("aria-expanded") === "true",
    { timeout, timeoutMsg: "第二个成员角色选择菜单未打开" });
    await press(await $('//*[@role="option" and normalize-space(.)="管理员"]'));
    await browser.waitUntil(async () => (await requestCount("update_project_member_role")) === roleCount + 2, { timeout });

    const respondNewest = (command) => browser.execute((name) => {
      const audit = window.__uiAudit;
      const index = audit.pending.findLastIndex((item) => item.command === name);
      if (index < 0) throw new Error("No deferred fixture request: " + name);
      const [request] = audit.pending.splice(index, 1);
      audit.deferred = audit.deferred.filter((item) => item !== name);
      request.resolve(Response.json(audit.fixtures.get_project));
    }, command);
    await browser.waitUntil(async () => (await requestCount("get_project")) === projectReadCount + 2, { timeout });
    await respondNewest("get_project");
    await respondNewest("get_project");
    await waitText($("body"), "成员角色已更新");
    await browser.waitUntil(async () => (await panel.getText()).includes("共 3 位成员"), { timeout });
    assert.equal(await browser.execute(() => document.body.innerText.includes("刷新失败")), false);
    assert.deepEqual(await browser.execute(() => window.__uiAudit.pending.map((item) => item.command)), []);
    assert.deepEqual(await snapshot(current.id, 1440, "latest-refresh", panel, {
      memberRoles: { "2": "viewer", "3": "manager" }, supersededProjectRefreshReported: false,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1867 does not apply a pending member confirmation after leaving its project", async function () {
    current = { id: "UX-1867-collaboration-confirm-scope", task: "切换项目后确认框不再提交旧项目的移除命令", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await collaborationPanel();
    const requestCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "remove_project_member").length,
    );
    const editorRow = await panel.$('.//li[.//div[contains(normalize-space(.),"协作编辑 · 长文本核对")]]');
    await press(await editorRow.$('button[aria-label="更多操作"]'));
    await menuItem("移除成员");
    const confirmation = await dialog("移除协作者");
    await browser.back();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/", { timeout });
    await clickText("移除", confirmation);
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "remove_project_member").length,
    ), requestCount);
    const homePage = await $(".a4n-home-content");
    assert.deepEqual(await snapshot(current.id, 1440, "confirmation-cancelled-by-scope", homePage, {
      route: "/", removeCommandCount: requestCount,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1871 exposes only member actions allowed by the current project role", async function () {
    current = { id: "UX-1871-collaboration-manager-access", task: "管理者不能提交服务端明确拒绝的 Manager 授权或成员移除", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await collaborationPanel(() => browser.execute(() => {
      const fixtures = window.__uiAudit.fixtures;
      fixtures.get_project.my_role = "manager";
      fixtures.projectRecords.find((project) => project.id === 1).my_role = "manager";
      fixtures.list_projects.items.find((project) => project.id === 1).my_role = "manager";
      fixtures.list_project_members = fixtures.list_project_members.map((member) =>
        member.user_id === 2 ? { ...member, role: "manager" } : member,
      );
    }));
    const inviteRole = await panel.$('button[aria-label="邀请角色"]');
    await press(inviteRole);
    const managerOption = await $('[role="option"][aria-disabled="true"]');
    await browser.waitUntil(async () =>
      (await browser.execute(() => document.body.innerText)).includes("管理员（仅所有者可授予）"),
      { timeout, timeoutMsg: "The disabled manager role must remain visible to the owner" },
    );
    assert.equal(
      await managerOption.execute((option) => option.textContent?.trim()),
      "管理员（仅所有者可授予）",
    );
    await browser.keys("Escape");
    const managerRow = await panel.$('.//li[.//div[normalize-space(.)="协作编辑 · 长文本核对"]]');
    assert.equal(await managerRow.$('button[aria-label^="调整"]').isExisting(), false);
    assert.equal(await managerRow.$('button[aria-label="更多操作"]').isExisting(), false);
    assert.deepEqual(await snapshot(current.id, 1440, "manager-access", panel, {
      restrictedRoleOptionDisabled: true, managerTargetActionsHidden: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2259 refreshes collaboration state when a member mutation is rejected", async function () {
    current = { id: "UX-2259-collaboration-stale-member", task: "另一位所有者先移除成员，本地删除命令失败后自动同步目录", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const panel = await collaborationPanel();
    const memberRow = await panel.$('.//li[.//div[contains(normalize-space(.),"协作编辑 · 长文本核对")]]');
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.fixtures.remove_project_member = null;
      audit.fixtures.list_project_members = audit.fixtures.list_project_members.filter((member) => member.user_id !== 2);
      audit.failNext.push({
        command: "remove_project_member", status: 404, message: "协作者已被其他用户移除",
      });
    });
    await press(await memberRow.$('button[aria-label="更多操作"]'));
    await menuItem("移除成员");
    const confirmation = await dialog("移除协作者");
    await clickText("移除", confirmation);
    await waitText($("body"), "协作者已被其他用户移除");
    await browser.waitUntil(async () =>
      (await panel.getText()).includes("共 2 位成员") && !(await memberRow.isExisting()),
    { timeout, timeoutMsg: "A rejected member action must reload the current collaboration directory" });
    assert.deepEqual(await snapshot(current.id, 1440, "external-removal-synchronized", panel, {
      memberNoLongerListed: true,
      failedLocalRemovalKeptItsError: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1744 keeps parent profile refresh failures at the dialog owner and retries that list", async function () {
    const requestCount = (command) => browser.execute((name) =>
      window.__uiAudit.requests.filter((item) => item.command === name).length, command);
    current = { id: "UX-1744", task: "Recover the parent character list after an attribute write", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const detail = await characterTab("管理", "属性");
    const maintain = await detail.$('.//details[.//summary[normalize-space(.)="手动维护属性"]]');
    await press(await maintain.$("summary"));
    await maintain.$('.//label[.//span[normalize-space(.)="属性名称"]]/input').setValue("UX-1744 parent refresh");
    await maintain.$('.//label[.//span[normalize-space(.)="原因"]]/textarea').setValue("验证父级列表刷新失败的状态归属");
    await maintain.$('.//label[.//span[normalize-space(.)="证据"]]/textarea').setValue("隔离 UI 夹具");

    const addCount = await requestCount("add_profile_attribute");
    const profileCount = await requestCount("list_profiles");
    await browser.execute(() => {
      window.__uiAudit.fixtures.add_profile_attribute = null;
      window.__uiAudit.failNext.push({
        command: "list_profiles", status: 503, message: "父级角色列表刷新失败",
      });
    });
    await press(await textButton("新增属性", maintain));
    await browser.waitUntil(async () => (await requestCount("add_profile_attribute")) === addCount + 1, { timeout });
    await waitText($("body"), "已新增属性");
    const parentError = await detail.$('.//*[@role="alert" and contains(normalize-space(.),"刷新失败")]');
    await parentError.waitForDisplayed({ timeout });
    assert.equal(await requestCount("list_profiles"), profileCount + 1);
    assert.deepEqual(await snapshot("UX-1744", 1440, "parent-refresh-error", detail, {
      writeSucceeded: true, parentRefresh: "failed", retryOwner: "character list",
    }), []);

    const historyCountBeforeRetry = await requestCount("attribute_history");
    await press(await textButton("重试", parentError));
    await browser.waitUntil(async () => !(await parentError.isDisplayed().catch(() => false)), { timeout });
    assert.equal(await requestCount("list_profiles"), profileCount + 2, "Retry must rerun the parent profile read");
    assert.equal(await requestCount("attribute_history"), historyCountBeforeRetry, "Parent retry must not reread the child event list");
    assert.deepEqual(await snapshot("UX-1744", 1440, "parent-refresh-recovered", detail, {
      parentRefresh: "retried successfully", childHistoryReads: historyCountBeforeRetry,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1775 owns manual fact-feedback refresh loading separately from polling", async function () {
    current = { id: "UX-1775-fact-feedback-refresh", task: "正文事实反馈手动刷新期间显示忙状态并阻止重复读取", visited: [], plannedWidths: [1440] };
    const page = await project();
    const panel = await page.$('.//section[./header/h3[normalize-space()="正文事实反馈"]]');
    await waitText(panel, "已归纳 2 章 · 待归纳 1/3 章");
    const refresh = await panel.$("button=刷新");
    const initialReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project_fact_feedback").length,
    );
    await browser.execute(() => window.__uiAudit.deferred.push("get_project_fact_feedback"));
    await press(refresh);
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "get_project_fact_feedback"),
    ));
    assert.equal(await refresh.isEnabled(), false, "A manual read in flight must own the refresh control");
    assert.equal(await panel.$(".animate-spin").isExisting(), true, "A manual read in flight must be visible");
    const readCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project_fact_feedback").length,
    );
    assert.equal(readCount - initialReads, 1, "A manual refresh must start one read regardless of dev StrictMode mounts");
    await browser.execute(() => {
      const fixtures = window.__uiAudit.fixtures;
      const index = window.__uiAudit.pending.findIndex((item) => item.command === "get_project_fact_feedback");
      const [pending] = window.__uiAudit.pending.splice(index, 1);
      pending.resolve(Response.json(fixtures.get_project_fact_feedback));
    });
    await browser.waitUntil(async () => refresh.isEnabled());
    current.visited.push(1440);
    current.evidence = { initialAndManualReads: readCount, pendingControlDisabled: true, completedReadRestoredControl: true };
  });

  it("UX-1776 keeps a slow fact-feedback poll from invalidating its own response", async function () {
    current = { id: "UX-1776-fact-feedback-poll", task: "正文事实反馈后台轮询不应重叠并丢弃慢回包", visited: [], plannedWidths: [1440] };
    const page = await project();
    const panel = await page.$('.//section[./header/h3[normalize-space()="正文事实反馈"]]');
    await waitText(panel, "已归纳 2 章 · 待归纳 1/3 章");
    const refresh = await panel.$("button=刷新");
    const initialReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project_fact_feedback").length,
    );
    await browser.execute(() => window.__uiAudit.deferred.push("get_project_fact_feedback"));
    await press(refresh);
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "get_project_fact_feedback"),
    ));
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const index = audit.pending.findIndex((item) => item.command === "get_project_fact_feedback");
      const [pending] = audit.pending.splice(index, 1);
      audit.fixtures.get_project_fact_feedback.workflow_status = "running";
      pending.resolve(Response.json(audit.fixtures.get_project_fact_feedback));
    });
    await browser.waitUntil(async () => refresh.isEnabled());
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "get_project_fact_feedback"),
    ), { timeout: 5000, timeoutMsg: "The running workflow must start its first background status read" });
    const pendingPolls = await browser.execute(() =>
      window.__uiAudit.pending.filter((item) => item.command === "get_project_fact_feedback").length,
    );
    await browser.pause(1800);
    const readsWhilePending = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project_fact_feedback").length,
    );
    const pendingPollsAfterInterval = await browser.execute(() =>
      window.__uiAudit.pending.filter((item) => item.command === "get_project_fact_feedback").length,
    );
    assert.equal(pendingPolls, 1);
    assert.equal(readsWhilePending - initialReads, 2, "Manual refresh and one background poll must be the only new reads");
    assert.equal(pendingPollsAfterInterval, 1, "A slow background read must not be overlapped by the next interval");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.deferred = audit.deferred.filter((command) => command !== "get_project_fact_feedback");
      audit.fixtures.get_project_fact_feedback.workflow_status = null;
      const index = audit.pending.findIndex((item) => item.command === "get_project_fact_feedback");
      const [pending] = audit.pending.splice(index, 1);
      pending.resolve(Response.json(audit.fixtures.get_project_fact_feedback));
    });
    current.visited.push(1440);
    current.evidence = { pendingPollsAfterInterval, readsWhilePending, slowPollKeptCurrent: true };
  });

  it("UX-2320 saves a fact-feedback interval while summary refresh is unavailable", async function () {
    current = { id: "UX-2320-fact-feedback-interval", task: "摘要刷新失败时仍可独立修改项目的事实反馈间隔", visited: [], plannedWidths: [1440] };
    const page = await project();
    const panel = await page.$('.//section[./header/h3[normalize-space()="正文事实反馈"]]');
    await waitText(panel, "已归纳 2 章 · 待归纳 1/3 章");
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "get_project_fact_feedback",
      status: 503,
      message: "事实反馈摘要读取失败",
    }));
    await press(await panel.$("button=刷新"));
    await waitText(panel, "事实反馈摘要读取失败");

    const interval = await panel.$('input[type="number"]');
    assert.equal(await interval.isEnabled(), true, "The project interval is independent of the feedback summary read");
    await interval.setValue("4");
    await browser.execute(() => {
      window.__uiAudit.fixtures.get_project_fact_feedback.interval_chapters = 4;
      window.__uiAudit.fixtures.update_project = window.__uiAudit.fixtures.get_project;
    });
    await press(await panel.$("button=保存周期"));
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.requests.some((item) =>
      item.command === "update_project" && item.body.input?.fact_feedback_interval_chapters === 4,
    )));
    await browser.waitUntil(async () => !(await panel.getText()).includes("事实反馈摘要读取失败"));
    await waitText(panel, "已归纳 2 章 · 待归纳 1/4 章");
    assert.equal(await panel.$('input[type="number"]').getValue(), "4");
    assert.deepEqual(await snapshot(current.id, 1440, "interval-saved-after-read-error", panel, {
      interval: 4,
      readRecovered: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1774 applies a default image config changed in another open workspace", async function () {
    current = { id: "UX-1774-image-config-sync", task: "跨窗口更换默认图片模型后，仍打开的场景工作区按新尺寸生成", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    await browser.execute(async () => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      const initial = local.listLocalImageConfigs().find((item) => item.name.startsWith("本机横幅"));
      if (!initial) throw new Error("Missing initial horizontal image fixture");
      await local.setDefaultLocalImageConfig(initial.id);
    });
    const workspace = await project("场景");
    await press(await workspace.$('button[aria-label="旧书店 · 查看场景详情"]'));
    const detail = await dialog();
    const sizeSelect = await detail.$('button[role="combobox"]');
    await browser.waitUntil(async () => (await sizeSelect.getText()).includes("1920 x 1080"), { timeout });
    const workspaceWindow = await browser.getWindowHandle();
    let settingsWindow;
    try {
      settingsWindow = (await browser.newWindow(new URL("/settings#image", browser.options.baseUrl).href)).handle;
      const initial = await $('//button[contains(normalize-space(.),"本机主用 · 角色立绘与场景参考")]');
      await $(".a4n-settings-layout").waitForDisplayed({ timeout: 20_000 });
      await initial.waitForDisplayed({ timeout: 20_000 });
      await press(initial);
      await clickText("设为默认");
      await waitText($("body"), "已设为默认");

      await browser.switchToWindow(workspaceWindow);
      await browser.waitUntil(async () => (await sizeSelect.getText()).includes("1024 x 1024"), {
        timeout,
        timeoutMsg: "The open scene workspace must adopt the new default size",
      });
      await browser.execute(() => window.__uiAudit.failNext.push({
        command: "start_image_generation", status: 503, message: "图片配置同步用例结束请求",
      }));
      await clickText("生成素材图", detail);
      await browser.waitUntil(() => browser.execute(() =>
        window.__uiAudit.requests.some((item) => item.command === "start_image_generation"),
      ));
      const operation = await browser.execute(() =>
        window.__uiAudit.requests.findLast((item) => item.command === "start_image_generation")?.body.operation,
      );
      assert.deepEqual({ width: operation.width, height: operation.height }, { width: 1024, height: 1024 });
      await waitText($("body"), "图片配置同步用例结束请求");
      current.visited.push(1440);
      current.evidence = { defaultChangedIn: "second window", selectedSize: "1024x1024", generatedOperation: operation };
    } finally {
      const handles = await browser.getWindowHandles();
      if (settingsWindow && handles.includes(settingsWindow)) {
        await browser.switchToWindow(settingsWindow);
        await browser.closeWindow();
      }
      if ((await browser.getWindowHandles()).includes(workspaceWindow)) {
        await browser.switchToWindow(workspaceWindow);
      }
    }
  });

  it("UX-1777 reports progress-read errors without failing the running scene sync", async function () {
    current = { id: "UX-1777-progress-read-error", task: "场景同步继续运行时显示进度读取错误", visited: [], plannedWidths: [1440] };
    const page = await project("场景");
    await press(await page.$('button[aria-haspopup="menu"]'));
    await menuItem("同步场景档案");
    const confirmation = await dialog("同步场景档案");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.fixtures.sync_scene_profiles = null;
      audit.fixtures.get_workflow_progress_events = { events: [], stream_text: null };
      audit.deferred.push("sync_scene_profiles", "get_workflow_progress_events");
      audit.failNext.push({
        command: "get_workflow_progress_events", status: 503, message: "工作流进度读取失败",
      });
    });
    await clickText("开始", confirmation);
    const progress = await dialog("同步场景档案");
    const progressAlert = await progress.$('[role="alert"]');
    await browser.waitUntil(async () => await progressAlert.isExisting()
      && (await progressAlert.getText()).includes("工作流进度读取失败"), {
      timeout,
      timeoutMsg: "A failed status read must be visible while scene sync continues",
    });
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "sync_scene_profiles") &&
      window.__uiAudit.pending.some((item) => item.command === "get_workflow_progress_events"),
    ));
    assert.equal(await progress.$('[role="progressbar"]').getAttribute("aria-valuenow"), "0");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const index = audit.pending.findIndex((item) => item.command === "sync_scene_profiles");
      const [pending] = audit.pending.splice(index, 1);
      audit.deferred = audit.deferred.filter((command) => command !== "sync_scene_profiles");
      pending.resolve(Response.json(null));
    });
    await waitText(progress, "任务已完成");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.deferred = audit.deferred.filter((command) => command !== "get_workflow_progress_events");
      const pendingReads = audit.pending.filter((item) => item.command === "get_workflow_progress_events");
      audit.pending = audit.pending.filter((item) => item.command !== "get_workflow_progress_events");
      for (const request of pendingReads) request.resolve(Response.json({ events: [], stream_text: null }));
    });
    await browser.waitUntil(async () => !(await progress.$('[role="alert"]').isExisting()));
    current.visited.push(1440);
    current.evidence = { progressReadErrorVisible: true, businessTaskRemainedPending: true, businessTaskCompleted: true };
  });

  it("UX-1778 explains when the same project action is already running", async function () {
    current = { id: "UX-1778-project-action-busy", task: "项目复制进行中再次点击时说明操作仍在执行", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    await home();
    const row = await rootRow();
    const openActions = () => row.$('button[aria-label="更多操作"]');
    await press(await openActions());
    await menuItem("复制为新项目");
    const prompt = await dialog("复制项目");
    await prompt.$("input").setValue("并发复制反馈测试");
    await browser.execute(() => {
      window.__uiAudit.fixtures.duplicate_project = null;
      window.__uiAudit.deferred.push("duplicate_project");
    });
    await clickText("复制", prompt);
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "duplicate_project"),
    ));
    await press(await openActions());
    await menuItem("复制为新项目");
    await waitText($("body"), "此项目操作正在进行，请等待完成。");
    const duplicateCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "duplicate_project").length,
    );
    assert.equal(duplicateCount, 1, "The second click must not start a duplicate request");
    assert.equal(await $('[role="dialog"]').isExisting(), false, "A blocked repeat action must not open another title prompt");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const index = audit.pending.findIndex((item) => item.command === "duplicate_project");
      const [pending] = audit.pending.splice(index, 1);
      audit.deferred = audit.deferred.filter((command) => command !== "duplicate_project");
      pending.resolve(Response.json({
        code: "unavailable", message: "模拟结束复制操作", retryable: true, trace_id: null, details: {},
      }, { status: 503 }));
    });
    await waitText($("body"), "模拟结束复制操作");
    current.visited.push(1440);
    current.evidence = { duplicateRequests: duplicateCount, repeatedActionFeedback: true };
  });

  it("UX-1782 keeps a committed project reset successful when local draft cleanup fails", async function () {
    current = { id: "UX-1782-reset-draft-cleanup", task: "项目数据重置成功后本地草稿清理失败应显示独立警告", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await writingPanel("写作助手");
    await press(await panel.$('button[aria-label="更多操作"]'));
    await menuItem("重置生成数据");
    const confirmation = await dialog("重置项目生成数据");
    const draftKey = "a4n:writing_draft:1:1";
    const draft = JSON.stringify({ content: "保留用于验证清理失败的本地草稿", updatedAt: new Date().toISOString() });
    await browser.execute(({ draftKey, draft, project }) => {
      window.__uiAudit.fixtures.reset_project_generated_data = {
        status: "completed",
        database_committed: true,
        result: {
          project,
          deleted_chapters: 3,
          deleted_chapter_outlines: 3,
          deleted_character_profiles: 0,
          preserved_blueprint_versions: 2,
        },
        warnings: [],
      };
      localStorage.setItem(draftKey, draft);
      window.__uiAudit.originalStorageRemoveItem = Storage.prototype.removeItem;
      Storage.prototype.removeItem = function (key) {
        if (key === draftKey) throw new Error("本地写作草稿删除失败");
        return window.__uiAudit.originalStorageRemoveItem.call(this, key);
      };
    }, { draftKey, draft, project: seed.commands.get_project });

    try {
      await clickText("重置", confirmation);
      await waitText($("body"), "本地写作草稿未能全部清理");
      const resetNotice = await $('[role="status"].a4n-toast');
      const resetMessage = await resetNotice.getText();
      assert.ok(resetMessage.includes("项目正式数据已重置"), "The committed reset must remain a completed operation");
      assert.ok(resetMessage.includes("本地写作草稿删除失败"), "The failed client-side cleanup must remain visible");
      assert.equal(await resetNotice.getAttribute("role"), "status", "A local cleanup warning must not be rendered as a failed reset");
      const resetRequests = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "reset_project_generated_data").length);
      assert.equal(resetRequests, 1, "The user action must issue one reset request");
      assert.equal(await browser.execute((key) => localStorage.getItem(key), draftKey), draft, "The injected storage failure must leave the draft available for diagnosis");
      current.visited.push(1440);
      current.evidence = { resetRequests, formalResetCommitted: true, cleanupErrorVisible: true, draftRemainedAfterCleanupFailure: true };
    } finally {
      await browser.execute((key) => {
        const original = window.__uiAudit.originalStorageRemoveItem;
        if (!original) return;
        Storage.prototype.removeItem = original;
        delete window.__uiAudit.originalStorageRemoveItem;
        original.call(localStorage, key);
      }, draftKey);
    }
  });

  it("UX-1783 waits for a pending native draft save before clearing project drafts", async function () {
    current = { id: "UX-1783-reset-pending-draft-save", task: "重置项目时等候已提交中的桌面草稿写入后再清理", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const draftKey = "a4n:writing_draft:1:1";
    await browser.execute(() => {
      for (let index = localStorage.length - 1; index >= 0; index -= 1) {
        const key = localStorage.key(index);
        if (key?.startsWith("a4n:writing_draft:1:")) localStorage.removeItem(key);
      }
      for (let index = sessionStorage.length - 1; index >= 0; index -= 1) {
        const key = sessionStorage.key(index);
        if (key?.startsWith("a4n:writing_draft:1:")) sessionStorage.removeItem(key);
      }
      const state = {};
      for (let index = 0; index < localStorage.length; index += 1) {
        const key = localStorage.key(index);
        const value = key?.startsWith("a4n:") ? localStorage.getItem(key) : null;
        if (key && value !== null) state[key] = value;
      }
      sessionStorage.setItem("__a4n_ui_audit_native_frontend_state", JSON.stringify(state));
    });
    nativeDraftStorageScenario = true;
    try {
      await writing();
      const editor = await $('textarea[aria-label="章节正文"]');
      await editor.setValue(seed.commands.list_chapters[0].content + " 桌面草稿写入尚未提交");
      await browser.waitUntil(() => browser.execute(() => window.__uiAudit.nativeStorage.pendingDraftWrite), {
        timeout,
        timeoutMsg: "The simulated native draft write must remain pending before reset",
      });
      await browser.execute((project) => {
        window.__uiAudit.fixtures.reset_project_generated_data = {
          status: "completed",
          database_committed: true,
          result: {
            project,
            deleted_chapters: 3,
            deleted_chapter_outlines: 3,
            deleted_character_profiles: 0,
            preserved_blueprint_versions: 2,
          },
          warnings: [],
        };
      }, seed.commands.get_project);

      await press(await $('.a4n-writing-panel-toggle button[title^="写作助手"]'));
      const panel = await $('aside[aria-label="写作助手"]');
      await panel.waitForDisplayed({ timeout });
      await press(await panel.$('button[aria-label="更多操作"]'));
      await menuItem("重置生成数据");
      await browser.waitUntil(() => browser.execute(() => window.__uiAudit.nativeStorage.pendingDraftWrite), {
        timeout,
        timeoutMsg: "Reset must wait for the first native draft save before asking for confirmation",
      });
      await browser.execute(() => {
        const release = window.__uiAudit.nativeStorage.releaseDraftWrite;
        if (!release) throw new Error("No pending native draft save to release before confirmation");
        release();
      });
      await browser.waitUntil(() => browser.execute(() => window.__uiAudit.nativeStorage.pendingDraftWrite), {
        timeout,
        timeoutMsg: "The reset confirmation's draft save must be queued after the initial native write",
      });
      await browser.execute(() => {
        const release = window.__uiAudit.nativeStorage.releaseDraftWrite;
        if (!release) throw new Error("No queued native draft save to release before confirmation");
        release();
      });
      await browser.waitUntil(
        () => browser.execute(() => [...document.querySelectorAll(".a4n-modal-layer")]
          .some((layer) => getComputedStyle(layer).display !== "none"
            && layer.innerText.includes("当前章节有未保存修改")
            && [...layer.querySelectorAll("button")].some((button) => getComputedStyle(button).display !== "none"))),
        { timeout, timeoutMsg: "The pending-edit confirmation must appear after the native draft save" },
      );
      await browser.execute(() => {
        // Use the running app's save queue; a source import has a separate storage singleton in preview.
        window.dispatchEvent(new Event("beforeunload", { cancelable: true }));
      });
      await browser.waitUntil(() => browser.execute(() => window.__uiAudit.nativeStorage.pendingDraftWrite), {
        timeout,
        timeoutMsg: "The reset cleanup must observe a second pending native draft save",
      });
      const nativeReadsBeforeReset = await browser.execute(() =>
        window.__uiAudit.nativeStorage.calls.filter((item) => item.command === "get_all_frontend_state").length,
      );
      const continueButton = await $('//*[contains(@class,"a4n-modal-layer")]//button[normalize-space(.)="继续"]');
      await continueButton.waitForDisplayed({ timeout });
      await continueButton.click();
      const confirmation = await dialog("重置项目生成数据");
      await clickText("重置", confirmation);
      await browser.waitUntil(() => browser.execute(() =>
        window.__uiAudit.requests.some((item) => item.command === "reset_project_generated_data"),
      ), { timeout });
      await browser.pause(100);

      const waitingState = await browser.execute(() => ({
        pendingDraftWrite: window.__uiAudit.nativeStorage.pendingDraftWrite,
        removedDraftKeys: [...window.__uiAudit.nativeStorage.removedDraftKeys],
        reads: window.__uiAudit.nativeStorage.calls.filter((item) => item.command === "get_all_frontend_state").length,
      }));
      assert.equal(waitingState.pendingDraftWrite, true);
      assert.deepEqual(waitingState.removedDraftKeys, [], "Reset must wait for the native save before enumerating keys");
      assert.equal(waitingState.reads, nativeReadsBeforeReset, "No stale cache snapshot may be used as the post-reset draft list");
      assert.equal(await $(".a4n-toast").isExisting(), false, "The reset result must wait for local draft deletion to finish");

      await browser.execute(() => {
        const release = window.__uiAudit.nativeStorage.releaseDraftWrite;
        if (!release) throw new Error("No pending native draft save to release");
        release();
      });
      await waitText($("body"), "已重置：删除 3 章");
      const finalState = await browser.execute((key) => ({
        exists: Object.hasOwn(window.__uiAudit.nativeStorage.state, key),
        removedDraftKeys: [...window.__uiAudit.nativeStorage.removedDraftKeys],
        reads: window.__uiAudit.nativeStorage.calls.filter((item) => item.command === "get_all_frontend_state").length,
      }), draftKey);
      assert.equal(finalState.exists, false, "A draft committed while reset was pending must be removed");
      assert.deepEqual(finalState.removedDraftKeys, [draftKey]);
      assert.ok(finalState.reads > nativeReadsBeforeReset, "The project draft list must come from the post-write native state");
      current.visited.push(1440);
      current.evidence = { waitingState, finalState, resetRequests: 1, pendingNativeWriteReleased: true };
    } finally {
      nativeDraftStorageScenario = false;
      await browser.execute(() => {
        const nativeStorage = window.__uiAudit?.nativeStorage;
        nativeStorage?.releaseDraftWrite?.();
        sessionStorage.removeItem("__a4n_ui_audit_native_frontend_state");
      });
    }
  });

  it("UX-1785 saves a new chapter from the import response without requiring a second index read", async function () {
    current = { id: "UX-1785-new-chapter-save-result", task: "新建章节保存后使用写命令回包，不把索引读取失败当成保存失败", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    await writing();
    await press(await $('button[aria-label="新建章节"]'));
    const prompt = await dialog("新建章节");
    const chapterNumber = await prompt.$("input");
    await chapterNumber.setValue("10");
    await clickText("进入章节", prompt);
    const editor = await $('textarea[aria-label="章节正文"]');
    await browser.waitUntil(async () => (await editor.getValue()) === "");
    const savedContent = "新建章节保存响应的权威正文。";
    await editor.setValue(savedContent);
    const imported = {
      ...seed.commands.list_chapters[0],
      id: 110,
      chapter_number: 10,
      title: "第10章 · 新线索",
      content: savedContent,
      edit_revision: "ux-1785-revision",
      derived_sync_status: "current",
    };
    await browser.execute((chapter) => {
      window.__uiAudit.fixtures.import_chapter = chapter;
      window.__uiAudit.fixtures.list_chapters = [
        ...window.__uiAudit.fixtures.list_chapters.filter((item) => item.id !== chapter.id),
        chapter,
      ];
      window.__uiAudit.failNext.push({
        command: "list_chapter_index", status: 503, message: "不应发生的章节索引回读",
      });
    }, imported);
    const indexReadsBefore = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "list_chapter_index",
    ).length);

    await clickText("保存修改");
    await waitText($("body"), "章节正文与派生资料已保存");
    assert.equal(await editor.getValue(), savedContent);
    const indexReadsAfter = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "list_chapter_index",
    ).length);
    assert.equal(indexReadsAfter, indexReadsBefore, "The mutation response already contains the saved chapter");
    assert.equal(await browser.execute(() => window.__uiAudit.failNext.some((item) => item.command === "list_chapter_index")), true);
    await browser.execute(() => {
      window.__uiAudit.failNext = window.__uiAudit.failNext.filter((item) => item.command !== "list_chapter_index");
    });
    current.evidence = { importResponseId: imported.id, chapterNumber: 10, contentApplied: true, redundantIndexReads: 0 };
    current.visited.push(1440);
  });

  it("UX-1786 keeps a committed chapter save successful when local draft deletion fails", async function () {
    current = { id: "UX-1786-chapter-save-draft-cleanup", task: "正文保存提交后本地草稿删除失败时保留正确提交结果", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    await writing();
    const editor = await $('textarea[aria-label="章节正文"]');
    const savedContent = seed.commands.list_chapters[0].content + " 正文保存与本地草稿分离。";
    const draftKey = "a4n:writing_draft:1:1";
    const updatedChapter = {
      ...seed.commands.list_chapters[0],
      content: savedContent,
      edit_revision: "ux-1786-revision",
    };
    await browser.execute(({ draftKey, chapter }) => {
      localStorage.setItem(draftKey, JSON.stringify({ content: chapter.content, updatedAt: new Date().toISOString() }));
      window.__uiAudit.fixtures.update_chapter_content = chapter;
      window.__uiAudit.originalStorageRemoveItem = Storage.prototype.removeItem;
      Storage.prototype.removeItem = function (key) {
        if (key === draftKey) throw new Error("本地草稿清理失败");
        return window.__uiAudit.originalStorageRemoveItem.call(this, key);
      };
    }, { draftKey, chapter: updatedChapter });

    try {
      await editor.setValue(savedContent);
      await clickText("保存修改");
      await waitText($("body"), "本地草稿清理失败");
      await browser.waitUntil(() => browser.execute(() => [...document.querySelectorAll('[role="status"].a4n-toast')]
        .some((toast) => (toast.textContent ?? "").includes("正文已保存")
          && (toast.textContent ?? "").includes("本地草稿清理失败"))), { timeout });
      assert.equal(await editor.getValue(), savedContent);
      assert.equal(await $('.a4n-writing-save-state').getAttribute("data-state"), "saved");
      assert.equal(await browser.execute((key) => localStorage.getItem(key) != null, draftKey), true);
      current.evidence = { saveCommandCommitted: true, cleanupWarningVisible: true, editorBaseline: "saved", draftRemained: true };
      current.visited.push(1440);
    } finally {
      await browser.execute((key) => {
        const original = window.__uiAudit.originalStorageRemoveItem;
        if (!original) return;
        Storage.prototype.removeItem = original;
        delete window.__uiAudit.originalStorageRemoveItem;
        original.call(localStorage, key);
      }, draftKey);
    }
  });

  it("UX-1745 reports a scene-list retry while keeping the last successful rows", async function () {
    const requestCount = (command) => browser.execute((name) =>
      window.__uiAudit.requests.filter((item) => item.command === name).length, command);
    current = { id: "UX-1745", task: "Show scene-list refresh progress while retaining known scenes", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await writingPanel("写作助手");
    await press(await panel.$('button[aria-label="更多操作"]'));
    await menuItem("场景管理");
    const scenes = await dialog("章节场景");
    await browser.waitUntil(async () => (await scenes.$$("li")).length > 0, { timeout });
    const listCount = await requestCount("list_chapter_scenes");
    await browser.execute(() => window.__uiAudit.failNext.push(
      { command: "generate_chapter_scene", status: 503, message: "场景生成失败" },
      { command: "list_chapter_scenes", status: 503, message: "场景列表读回失败" },
    ));
    await press(await textButton("生成场景", scenes));
    await browser.waitUntil(async () => (await requestCount("list_chapter_scenes")) === listCount + 1, { timeout });
    const loadError = await scenes.$('.//*[@role="alert" and contains(normalize-space(.),"场景列表读回失败")]');
    await loadError.waitForDisplayed({ timeout });
    const generationFailure = await dialog("生成场景 9");
    await clickText("关闭", generationFailure);
    const knownRows = await scenes.$$("li");
    const knownTitles = await Promise.all(Array.from(knownRows).map(async (row) => row.$(".text-sm").getText()));

    await browser.execute(() => window.__uiAudit.deferred.push("list_chapter_scenes"));
    await press(await textButton("重试", loadError));
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "list_chapter_scenes"),
    ), { timeout });
    await waitText(scenes, "加载场景…");
    assert.equal(await scenes.$('[role="alert"]').isExisting(), false);
    assert.deepEqual(await Promise.all(Array.from(await scenes.$$("li")).map(async (row) => row.$(".text-sm").getText())), knownTitles);
    assert.deepEqual(await snapshot("UX-1745", 1440, "retry-in-flight", scenes, {
      knownRows: knownTitles.length, refreshRequest: "pending", loadingFeedback: true,
    }), []);

    await browser.execute(() => {
      const pending = window.__uiAudit.pending.findLast((item) => item.command === "list_chapter_scenes");
      if (!pending) throw new Error("Expected the deferred scene-list refresh");
      window.__uiAudit.pending = window.__uiAudit.pending.filter((item) => item !== pending);
      window.__uiAudit.deferred = window.__uiAudit.deferred.filter((command) => command !== "list_chapter_scenes");
      pending.resolve(Response.json(window.__uiAudit.fixtures.byChapter[1].list_chapter_scenes));
    });
    await browser.waitUntil(async () => !(await scenes.$('[role="status"]').isExisting()), { timeout });
    assert.equal(await scenes.$$("li").length, knownTitles.length);
    assert.deepEqual(await snapshot("UX-1745", 1440, "retry-complete", scenes, {
      knownRows: knownTitles.length, refreshRequest: "succeeded",
    }), []);
    current.visited.push(1440);
  });

  it("AUDIT-export-download writes the TXT file returned by the export command", async function () {
    current = { id: "AUDIT-export-download", task: "Download and inspect the actual TXT export artifact", visited: [], plannedWidths: [1440] };
    await mkdir(downloadOutput, { recursive: true });
    await browser.sendCommand("Browser.setDownloadBehavior", {
      behavior: "allow",
      downloadPath: downloadOutput,
    });
    await browser.setWindowSize(1440, height);
    const exportDialog = await openProjectAction("导出小说");
    const fileName = `a4n-export-audit-${Date.now()}.txt`;
    const content = "A4N export path fixture";
    await browser.execute(({ nextFileName, nextContent }) => {
      window.__uiAudit.fixtures.export_txt_data = {
        file_name: nextFileName,
        mime_type: "text/plain;charset=utf-8",
        data_base64: btoa(nextContent),
      };
    }, { nextFileName: fileName, nextContent: content });

    await press(await textButton("下载 TXT 纯文本", exportDialog));
    const request = await browser.execute(() => window.__uiAudit.requests.findLast(
      (item) => item.command === "export_txt_data",
    ));
    assert.equal(request?.body.input.project_id, 1, "The export must target the project opened by the user");
    await browser.waitUntil(async () => (await readdir(downloadOutput)).includes(fileName), {
      timeout,
      timeoutMsg: "The TXT export command completed without creating its download file",
    });
    await waitText(exportDialog, "下载完成");
    assert.equal(await readFile(join(downloadOutput, fileName), "utf8"), content);
    assert.deepEqual(await snapshot(current.id, 1440, "downloaded-content", exportDialog, {
      fileName, bytes: Buffer.byteLength(content), responseAndFileMatch: true,
    }), []);
    current.visited.push(1440);
  });

  const pages = [
    { id: "N00-home", task: "找到原项目、派生项目和继续创作入口", open: home, text: [rootTitle], last: async () => (await $$(".a4n-project-row")).at(-1) },
    { id: "N01-overview", tab: "概览", task: "核对项目概况与正式蓝图", text: ["一封未寄出的信", "蓝图版本"], check: assertProjectTitleVisible },
    { id: "N02-world", tab: "世界观", task: "阅读世界规则、地点与阵营", text: ["所有异常都必须依附", "旧书店", "书店守望者", "优化世界观"], check: assertProjectTitleVisible },
    { id: "N03-characters", tab: "角色", task: "查找12个角色并打开档案", text: ["林岚", "周衡", "沈照"], check: assertProjectTitleVisible, last: async (scope) => scope.$('button[aria-label^="沈照 ·"]') },
    { id: "N04-objects", tab: "物体", task: "查找道具及其来源", text: ["旧怀表", "未寄出的信", "褪色梧桐叶"], last: async (scope) => scope.$('button[aria-label^="褪色梧桐叶 ·"]') },
    { id: "N05-scenes", tab: "场景", task: "查找地点及跨章引用", text: ["旧书店", "阁楼", "沿海邮路档案室"], last: async (scope) => scope.$('button[aria-label^="沿海邮路档案室 ·"]') },
    { id: "N06-relationships", tab: "关系", task: "阅读有节点与连线的人物关系图", text: ["林岚", "周衡"], check: async (scope) => {
      const graph = await relationshipGraph(scope, "initialGraph");
      const panels = await scope.execute((root) => {
        const graph = root.querySelector(".react-flow");
        const list = root.querySelector("aside");
        if (!graph || !list) throw new Error("The relationship graph and character list must both exist");
        const rect = (element) => {
          const r = element.getBoundingClientRect();
          return { left: r.left, top: r.top, right: r.right, bottom: r.bottom, width: r.width, height: r.height };
        };
        const graphRect = rect(graph), listRect = rect(list);
        const grid = graph.parentElement.parentElement;
        const style = getComputedStyle(grid);
        return {
          graph: graphRect, list: listRect, grid: rect(grid), gridRows: style.gridTemplateRows,
          overlapWidth: Math.max(0, Math.min(graphRect.right, listRect.right) - Math.max(graphRect.left, listRect.left)),
          overlapHeight: Math.max(0, Math.min(graphRect.bottom, listRect.bottom) - Math.max(graphRect.top, listRect.top)),
        };
      });
      const evidence = { relationshipNodes: graph.nodes, relationshipEdges: graph.edgeLengths.length, panels };
      current.evidence = evidence;
      assert.ok(panels.overlapWidth <= 1 || panels.overlapHeight <= 1, "Relationship graph must not cover the character list: " + JSON.stringify(panels));
      return evidence;
    }, last: async (scope) => scope.$('aside button[title="白砚"]') },
    { id: "N07-events", tab: "事件", task: "直接找到章节事件及筛选入口", text: [seed.commands.list_chapters[0].analysis_data.key_events[0].description], check: async (scope) => {
      assert.equal(await scope.$(".content-main > details").getProperty("open"), false, "The foreshadowing index starts collapsed");
      const search = await wheelTo(await scope.$('input[placeholder="搜索事件、章节、地点或角色"]'));
      assert.ok(search.reached);
      assert.equal(search.wheelSteps, 0, "Event search must be discoverable in the initial viewport");
      return { foreshadowingCollapsed: true, eventSearch: search };
    } },
    { id: "N08-parts", tab: "大纲", task: "查看三部分结构", open: async () => {
      const scope = await project("大纲"); await clickText("分部大纲", scope); return scope;
    }, text: ["夜雨中的旧书店", "通道内外的双重档案", "尚未写完的归途"] },
    { id: "N09-images", tab: "图像", task: "查找有内容的图像与收藏", check: async (scope) => {
      await browser.waitUntil(async () => (await scope.$$("img")).length >= 12, { timeout, timeoutMsg: "Image gallery has not finished loading" });
      const images = await scope.$$("img"); assert.ok(images.length >= 12, "Populated image gallery");
      await browser.waitUntil(async () => images[0].execute((image) => image.complete && image.naturalWidth > 0), { timeout });
      return { images: images.length };
    }, last: async (scope) => (await scope.$$("img")).at(-1) },
    { id: "N10-imports", tab: "导入/归纳", task: "从有分析结果的审核队列定位材料", text: ["01-旧书店", "12-旧书店"], last: async (scope) => scope.$('button[aria-label*="12-旧书店"]') },
    { id: "N11-chapters", tab: "章节", task: "查找已生成章节并进入正文", text: ["雨夜来信", "纸上的地图", "黎明的站台"], last: async (scope) => containing("黎明的站台", scope) },
    { id: "N12-cost", tab: "成本", task: "区分计价、未计价与失败请求", text: ["USD", "CNY"], check: async (scope) => {
      const rows = (await scope.$$("tbody tr")).length;
      assert.equal(rows, seed.commands.get_model_usage_report.recent.length, "All returned cost records must be present");
      const headers = await Promise.all([...await scope.$$("thead th")].map((item) => item.getText()));
      assert.deepEqual(headers.slice(0, 2), ["金额", "状态 / 置信度"]);
      return { returnedRows: seed.commands.get_model_usage_report.recent.length, visibleRows: rows, priorityColumns: headers.slice(0, 2) };
    }, last: async (scope) => (await scope.$$("tbody tr:last-child td")).at(-1) },
  ];
  for (const page of pages) it(page.id + " follows its real entry and checks all widths", async () => matrix({
    ...page, open: page.open ?? (() => project(page.tab)),
  }));

  it("N03 keeps portrait and profile read failures distinct from empty results", async () => {
    current = { id: "N03-portrait-read-errors", task: "区分立绘列表和角色档案读取失败与真实空列表", visited: [], plannedWidths: [390] };
    await browser.setWindowSize(390, height);
    for (const failure of [
      { command: "list_portraits_by_project", message: "立绘列表服务暂不可用" },
      { command: "list_profiles", message: "角色档案服务暂不可用" },
    ]) {
      const page = await project("角色");
      await clickText("档案", page);
      await waitText(page, "林岚");
      await browser.execute((nextFailure) => {
        window.__uiAudit.failNext.push(...Array.from({ length: 4 }, () => ({
          ...nextFailure,
          status: 503,
        })));
      }, failure);
      await clickText("立绘", page);
      await waitText(page, failure.message);
      if (failure.command === "list_portraits_by_project") {
        assert.equal(await page.$$('button[title="管理人物三视图拆解"]').length, 0);
        assert.equal((await page.getText()).includes("尚未生成人物素材"), false);
      } else {
        assert.equal((await page.getText()).includes("角色档案读取失败"), true);
        const cards = await page.$$('button[title="管理人物三视图拆解"]');
        assert.equal(cards.length, 4);
        assert.equal((await cards[0].getText()).includes("角色档案读取失败"), true);
      }
    }
    current.visited.push(390);
  });

  it("N03 reports reference-image read failures instead of an empty library", async () => {
    current = { id: "N03-reference-read-error", task: "角色立绘生成时区分参考图库读取失败与真实空图库", visited: [], plannedWidths: [390] };
    await browser.setWindowSize(390, height);
    const page = await project("角色");
    await clickText("立绘", page);
    await waitText(page, "立绘素材");
    await press(await page.$('button[aria-label="更多操作"]'));
    await browser.execute(() => window.__uiAudit.failNext.push(...Array.from({ length: 4 }, () => ({
      command: "list_images_by_project",
      status: 503,
      message: "参考图服务暂不可用",
    }))));
    await menuItem("生成单个角色立绘");
    const form = await dialog("生成人物三视图拆解");
    await waitText(form, "参考图服务暂不可用");
    assert.equal((await form.getText()).includes("当前项目尚无图片"), false);
    await closeDialog();
    current.visited.push(390);
  });

  it("N03 keeps a successful portrait upload successful when its list refresh fails", async () => {
    current = { id: "N03-portrait-upload-readback", task: "上传立绘成功后列表读回失败，仍明确反馈写入结果", visited: [], plannedWidths: [390] };
    await browser.setWindowSize(390, height);
    const page = await portraits();
    await browser.execute(() => {
      const portrait = window.__uiAudit.fixtures.byProject[1].list_portraits_by_project[0];
      window.__uiAudit.fixtures.upload_portrait = {
        ...portrait,
        id: 1199,
        image_path: "https://ui.invalid/image-1199.png",
        prompt: "fixture upload result",
      };
      window.__uiAudit.failNext.push({
        command: "list_portraits_by_project",
        status: 503,
        message: "立绘列表刷新失败",
      });
    });
    await press(await page.$('button[aria-label="更多操作"]'));
    await menuItem("上传立绘素材");
    const form = await dialog("上传角色立绘");
    const profile = await form.$('button[aria-label="角色档案"]');
    await press(profile);
    await press(await $('//*[@role="option" and normalize-space(.)="林岚"]'));
    const file = await form.$('input[type="file"]');
    await file.setValue(fileURLToPath(new URL("../../resources/thelittleprince.png", import.meta.url)));
    const prompt = await form.$("textarea");
    assert.match(await prompt.getValue(), /林岚.*thelittleprince\.png/);
    await press(profile);
    await press(await $('//*[@role="option" and normalize-space(.)="周衡"]'));
    assert.match(await prompt.getValue(), /周衡.*thelittleprince\.png/);
    await file.execute((node) => {
      const transfer = new DataTransfer();
      transfer.items.add(new File([new Uint8Array([1])], "replacement.png", { type: "image/png" }));
      node.files = transfer.files;
      node.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await browser.waitUntil(async () => (await prompt.getValue()).includes("replacement.png"), { timeout });
    assert.match(await prompt.getValue(), /周衡.*replacement\.png/);
    await prompt.setValue("周衡在雨夜书店中的手工描述。");
    await press(profile);
    await press(await $('//*[@role="option" and normalize-space(.)="林岚"]'));
    await file.execute((node) => {
      const transfer = new DataTransfer();
      transfer.items.add(new File([new Uint8Array([2])], "final.png", { type: "image/png" }));
      node.files = transfer.files;
      node.dispatchEvent(new Event("change", { bubbles: true }));
    });
    assert.equal(await prompt.getValue(), "周衡在雨夜书店中的手工描述。");
    await press(await textButton("上传立绘", form));
    await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"]')]
      .some((item) => item.textContent?.includes("上传角色立绘"))), { timeout });
    await waitText(page, "立绘列表刷新失败");
    await waitText($("body"), "已上传角色立绘");
    const uploadRequest = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "upload_portrait"));
    assert.equal(uploadRequest?.body.input.profile_id, 1011);
    assert.equal(uploadRequest?.body.input.file_name, "final.png");
    assert.equal(uploadRequest?.body.input.prompt, "周衡在雨夜书店中的手工描述。");
    current.visited.push(390);
  });

  it("UX-2203 uses the active interface language for the portrait upload suggestion", async () => {
    current = { id: "UX-2203", task: "英语界面上传立绘时默认描述保持英文", visited: [], plannedWidths: [390] };
    await browser.setWindowSize(390, height);
    const page = await project("角色");
    await switchInterfaceLanguage("English", "en-US", "界面语言");
    await clickText("Portraits", page);
    await press(await page.$('button[aria-label="More actions"]'));
    await menuItem("Upload portrait material");
    const form = await dialog("Upload character portrait");
    await form.$('input[type="file"]').setValue(fileURLToPath(new URL("../../resources/thelittleprince.png", import.meta.url)));
    assert.equal(
      await form.$("textarea").getValue(),
      "User-uploaded character portrait: 林岚; source file: thelittleprince.png",
    );
    await browser.keys("Escape");
    const discard = await dialog("Unsaved changes");
    await press(await containing("Discard draft", discard));
    await switchInterfaceLanguage("简体中文", "zh-CN", "Interface language");
    current.visited.push(390);
  });

  it("UX-B73 keeps empty number drafts editable and validates chapter import", async function () {
    const widthsToCheck = [1440, 390];
    for (const width of widthsToCheck) {
      await browser.setWindowSize(width, height);

      let page = await project("大纲");
      await clickText("分部大纲", page);
      await clickText("直接录入", page);
      let form = await dialog();
      const partNumberFields = await form.$$('input[type="number"]');
      assert.equal(partNumberFields.length, 3);
      for (const field of partNumberFields) {
        const original = await field.getValue();
        await clearByKeyboard(field);
        assert.equal(await field.getValue(), "", "A cleared part number must remain empty while editing");
        await field.setValue(original);
        assert.equal(await field.getValue(), original);
      }
      await clickText("取消", form);

      page = await project("大纲");
      await clickText("章节大纲", page);
      await clickText("直接录入", page);
      form = await dialog();
      const chapterNumberField = await form.$('input[type="number"]');
      const initialChapterNumber = await chapterNumberField.getValue();
      await clearByKeyboard(chapterNumberField);
      assert.equal(await chapterNumberField.getValue(), "", "A cleared chapter number must remain empty while editing");
      await form.$('button[type="submit"]').click();
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("save_chapter_outline")), false,
        "The form must not submit an empty required chapter number");
      await chapterNumberField.setValue(initialChapterNumber);
      await clickText("取消", form);

      page = await project("大纲");
      await clickText("章节大纲", page);
      const generationDetails = await page.$("section details.a4n-flat-surface.group");
      if (!(await generationDetails.getProperty("open"))) await generationDetails.$("summary").click();
      const countField = await page.$('input[type="number"][max="50"]');
      const initialCount = await countField.getValue();
      await clearByKeyboard(countField);
      assert.equal(await countField.getValue(), "", "An empty chapter generation count must remain editable");
      await countField.setValue("12");
      assert.equal(await countField.getValue(), "12");
      await browser.saveScreenshot(output + phase + "-b73-number-inputs-" + width + ".png");
      await countField.setValue(initialCount);

      page = await project("章节");
      await clickText("导入章节", page);
      form = await dialog();
      const importChapterNumber = await form.$('input[type="number"]');
      const initialImportNumber = await importChapterNumber.getValue();
      await clearByKeyboard(importChapterNumber);
      assert.equal(await importChapterNumber.getValue(), "", "A cleared import chapter number must remain empty");
      await form.$("textarea").setValue("用于验证章节号边界的正文");
      const confirmImport = await textButton("确认导入", form);
      assert.equal(await confirmImport.isEnabled(), true, "The action must reach its explicit chapter-number validation");
      await confirmImport.click();
      await browser.waitUntil(async () => {
        const alerts = await browser.$$('[role="alert"]');
        for (const alert of alerts) {
          if (await alert.isDisplayed() && (await alert.getText()).includes("章节号必须是 1 到 2,147,483,647 之间的整数")) return true;
        }
        return false;
      }, {
        timeout, timeoutMsg: "An invalid chapter number must produce visible feedback",
      });
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("import_chapter")), false,
        "An invalid chapter number must not reach the import command");
      await form.$("textarea").clearValue();
      await importChapterNumber.setValue(initialImportNumber);

      const deletion = await characterTab("管理", "删除");
      const deletionRequest = await deletion.$('//details[.//summary[normalize-space(.)="请求删除属性"]]');
      await deletionRequest.$("summary").click();
      const deletionChapter = await deletionRequest.$('input[type="number"]');
      const initialDeletionChapter = await deletionChapter.getValue();
      await clearByKeyboard(deletionChapter);
      assert.equal(await deletionChapter.getValue(), "", "A cleared deletion chapter must remain editable");
      await deletionChapter.setValue("12");
      assert.equal(await deletionChapter.getValue(), "12");
      await deletionChapter.setValue(initialDeletionChapter);
    }
  });

  it("UX-1670 rejects invalid part sizes before request preview", async function () {
    current = { id: "UX-1670-part-preview-validation", task: "预览分部大纲请求前校验每部章数", visited: [], plannedWidths: [1440, 390] };
    for (const width of current.plannedWidths) {
      current.width = width;
      await browser.setWindowSize(width, height);
      const page = await project("大纲");
      await clickText("分部大纲", page);
      const settings = await page.$("section details.a4n-flat-surface.group");
      if (!(await settings.getProperty("open"))) await settings.$("summary").click();
      const fields = await settings.$$('input[type="number"]');
      assert.equal(fields.length, 2);
      for (const value of ["1.5", "2147483648"]) {
        await fields[1].setValue(value);
        const before = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "preview_generate_parts_request").length);
        await press(await textButton("查看追加请求", settings));
        await waitText($("body"), "每部章数必须为正整数");
        assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "preview_generate_parts_request").length), before,
          "Invalid chapters_per_part must not reach the preview API");
        assert.deepEqual(await snapshot(current.id, width, `invalid-${value}`, page, { value }), []);
      }
      await fields[1].setValue("6");
      current.visited.push(width);
    }
  });

  it("UX-1671 keeps the successful write response visible when outline readback fails", async function () {
    current = { id: "UX-1671-outline-partial-read", task: "写入成功后章纲读回失败仍显示命令回包中的新增记录", visited: [], plannedWidths: [1440, 390] };
    for (const width of current.plannedWidths) {
      current.width = width;
      await browser.setWindowSize(width, height);
      const page = await project("大纲");
      await clickText("章节大纲", page);
      await waitText(page, seed.commands.list_chapter_outlines[0].title);
      const created = {
        ...seed.commands.list_chapter_outlines[0],
        id: 990,
        chapter_number: 10,
        title: "保留的新增章纲",
        summary: "写入已成功，列表读失败时仍显示最近一次成功读取。",
      };
      await browser.execute((outline, existing) => {
        window.__uiAudit.fixtures.save_chapter_outline = { outline, warnings: [] };
        window.__uiAudit.fixtures.list_chapter_outlines = [...existing, outline];
        window.__uiAudit.fixtures.list_chapter_outline_index = [...existing, outline];
        window.__uiAudit.failNext.push({
          command: "list_chapter_outline_index",
          status: 503,
          message: "章纲目录读取失败",
        });
      }, created, seed.commands.list_chapter_outlines);
      await clickText("直接录入", page);
      const form = await dialog();
      await form.$('input[type="text"]').setValue(created.title);
      await form.$("textarea").setValue(created.summary);
      await press(await form.$('button[type="submit"]'));
      await waitText(page, "章纲目录读取失败");
      await waitText(page, seed.commands.list_chapter_outlines[0].title);
      await waitText(page, created.title);
      assert.equal(await page.$$(".chapter-outline-open-indicator").length, seed.commands.list_chapter_outlines.length + 1);
      assert.deepEqual(await snapshot(current.id, width, "read-error-keeps-write-result", page, { previousRows: seed.commands.list_chapter_outlines.length, writtenOutline: created.title }), []);
      const alert = await page.$('[role="alert"]');
      await press(await textButton("重试", alert));
      await waitText(page, created.title);
      assert.deepEqual(await snapshot(current.id, width, "retry-loads-new-list", page, { newOutline: created.title }), []);
      current.visited.push(width);
    }
  });

  it("UX-1672 verifies the generated image remains successful when object profiles fail to refresh", async function () {
    current = { id: "UX-1672-object-image-profile-error", task: "物体立绘生成成功后物体档案读取失败仍显示写入结果", visited: [], plannedWidths: [1440, 390] };
    for (const [index, width] of current.plannedWidths.entries()) {
      current.width = width;
      await browser.setWindowSize(width, height);
      const page = await project("图像");
      await clickText("生成图片", page);
      const form = await dialog("生成项目图像");
      await press(await form.$("summary"));
      await press(await form.$("#object-image-profile"));
      await press(await $('//*[@role="option" and normalize-space(.)="旧怀表"]'));

      const sourceImage = seed.commands.list_images_by_projects.find((image) => image.metadata_json?.object_profile_id === 2011);
      assert(sourceImage, "The result verification needs an existing object-profile image shape");
      const image = {
        ...sourceImage,
        id: 50101 + index,
        image_type: "object",
        chapter_id: null,
        file_path: `${browser.options.baseUrl}/resources/thelittleprince.png`,
        prompt: `UI audit generated object image ${index}`,
        created_at: "2026-10-10T00:00:00Z",
      };
      const taskId = index === 0
        ? "550e8400-e29b-41d4-a716-446655440101"
        : "550e8400-e29b-41d4-a716-446655440102";
      const revision = await browser.execute(async (record) => {
        const { sha256Json } = await import("/src/utils/contentHash.ts");
        return sha256Json([
          record.id, record.project_id, record.image_type, record.file_path, record.prompt,
          record.model, record.width, record.height, record.file_size, record.chapter_id,
          record.panel_id, record.created_at,
        ]);
      }, image);
      assert.equal(typeof revision, "string");
      await browser.execute(({ id, record, recordRevision }) => {
        const handle = { event_schema_version: 1, task_id: id, attempt: 1, status: "reserved" };
        const started = {
          delivery_mode: "stream", root_task_id: id, task_kind: "image_generation",
          project_id: record.project_id, root_project_id: record.project_id,
          target: { kind: "generated_image", id: String(record.id) },
        };
        const artifact = {
          artifact_id: `generated-image:${record.id}`, artifact_kind: "generated_image",
          byte_size: record.file_size, content_type: "image/png", revision: recordRevision,
          sha256: recordRevision, temporary: false,
        };
        const resultRef = { kind: "generated_image", id: String(record.id), revision: recordRevision };
        const now = Date.now();
        const event = (sequence, value) => ({
          schema_version: 1, task_id: id, attempt: 1, sequence, emitted_at_ms: now + sequence, event: value,
        });
        const startedEvent = event(1, { kind: "started", data: started });
        const artifactEvent = event(2, { kind: "artifact", data: { artifact, operation: "available" } });
        const completedEvent = event(3, { kind: "completed", data: { final_text_hashes: [], result_ref: resultRef } });
        const completedHandle = { ...handle, status: "completed" };
        const summary = {
          handle: completedHandle, started, updated_at_ms: now + 3, result_ref: resultRef,
          snapshot_state: "available", terminal_event: completedEvent,
        };
        const snapshot = {
          schema_version: 1, task_id: id, attempt: 1, covers_through_sequence: 3, status: "completed",
          started, cancel_reason: null, progress: null, streams: [], artifacts: [artifact], usage: null,
          latest_checkpoint: null, terminal_event: completedEvent, updated_at_ms: now + 3,
        };
        const fixtures = window.__uiAudit.fixtures;
        fixtures.start_image_generation = handle;
        fixtures.generationTasks = {
          ...(fixtures.generationTasks ?? {}),
          [id]: { handle, events: [startedEvent, artifactEvent, completedEvent], summary, snapshot },
        };
        fixtures.byProject[1].list_images_by_project = [...fixtures.byProject[1].list_images_by_project, record];
        window.__uiAudit.failNext.push({ command: "list_object_profile_index", status: 503, message: "物体档案列表读取失败" });
      }, { id: taskId, record: image, recordRevision: revision });

      const objectTools = await form.$("details[open] section");
      await press(await textButton("生成素材图", objectTools));
      const progress = await dialog("生成物体素材图");
      await progress.waitForDisplayed({ timeout });
      const closeProgress = await progress.$("button=关闭");
      await closeProgress.waitForDisplayed({ timeout });
      await browser.waitUntil(() => browser.execute(() =>
        [...document.querySelectorAll(".a4n-toast")].some((node) => node.innerText.includes("生成物体素材图")),
      ), { timeout, timeoutMsg: "The completed image generation must report success before the toast expires" });
      await press(closeProgress);
      await browser.waitUntil(
        () => browser.execute(() => ![...document.querySelectorAll('[role="dialog"]')].some((node) => {
          const labelledBy = node.getAttribute("aria-labelledby");
          return labelledBy && document.getElementById(labelledBy)?.textContent?.trim() === "生成物体素材图";
        })),
        { timeout, timeoutMsg: "The object-image progress dialog must close without closing the generation form" },
      );
      await waitText(form, "物体档案列表读取失败");
      const generated = await page.$(`img[alt=${JSON.stringify(image.prompt)}]`);
      await generated.waitForExist({ timeout });
      assert.equal((await form.getText()).includes("图片生成失败"), false);
      await snapshot(current.id, width, "profile-read-error", page, { resultImageId: image.id, profileRead: "failed" });
      await press(await form.$('button[aria-label="关闭"]'));
      await form.waitForExist({ reverse: true });
      await generated.waitForDisplayed({ timeout });
      await browser.waitUntil(() => generated.execute((node) => node.complete && node.naturalWidth > 0), { timeout });
      assert.equal(await generated.isDisplayed(), true, "The saved result must be visible after closing the generation form");
      assert.deepEqual(await snapshot(current.id, width, "generated-result-visible", page, { resultImageId: image.id }), []);
      current.visited.push(width);
    }
  });

  it("UX-1677 keeps a created object profile when the following list read fails", async function () {
    current = { id: "UX-1677-object-profile-create-read-failure", task: "物体档案创建回包成功后列表读取失败仍保留新档案和成功反馈", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("物体");
    await waitText(page, "旧怀表");
    const created = {
      ...structuredClone(seed.commands.list_object_profiles[0]),
      id: 2099,
      name: "界面审查新物件",
      normalized_name: "界面审查新物件",
      ownership_kind: "user",
      aliases_json: [],
      evidence_json: [],
      generated_image_count: 0,
    };
    await browser.execute(({ profile }) => {
      window.__uiAudit.fixtures.create_object_profile = { profile, warnings: [] };
      window.__uiAudit.failNext.push({ command: "list_object_profiles", status: 503, message: "物体档案目录读取失败" });
    }, { profile: created });
    await clickText("新建档案", page);
    const form = await dialog("新建物体档案");
    await form.$('.//label[normalize-space(.)="名称"]/input').setValue(created.name);
    await clickText("保存", form);
    const detail = await dialog(created.name);
    await waitText(detail, created.name);
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("物体档案已创建"), { timeout });
    await press(await detail.$('button[aria-label="关闭"]'));
    await waitText(page, "物体档案目录读取失败");
    await waitText(page, created.name);
    assert.equal(await browser.execute(() => window.__uiAudit.unexpected.length), 0);
    assert.deepEqual(await snapshot(current.id, 1440, "write-result-kept", page, { createdId: created.id, refresh: "failed" }), []);
    current.visited.push(1440);
  });

  it("UX-1681 keeps a created scene profile when the following list read fails", async function () {
    current = { id: "UX-1681-scene-profile-create-read-failure", task: "场景档案创建回包成功后列表读取失败仍保留新档案和成功反馈", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("场景");
    await waitText(page, "旧书店");
    const created = {
      ...structuredClone(seed.commands.list_scene_profiles[0]),
      id: 99,
      name: "界面审查新场景",
      normalized_name: "界面审查新场景",
      ownership_kind: "user",
      aliases_json: [],
      evidence_json: [],
      generated_image_count: 0,
    };
    await browser.execute(({ profile }) => {
      window.__uiAudit.fixtures.create_scene_profile = { profile, warnings: [] };
      window.__uiAudit.failNext.push({ command: "list_scene_profiles", status: 503, message: "场景档案目录读取失败" });
    }, { profile: created });
    await clickText("新建档案", page);
    const form = await dialog("新建场景档案");
    await form.$('.//label[normalize-space(.)="名称"]/input').setValue(created.name);
    await clickText("保存", form);
    const detail = await dialog(created.name);
    await waitText(detail, created.name);
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("场景档案已创建"), { timeout });
    await press(await detail.$('button[aria-label="关闭"]'));
    await waitText(page, "场景档案目录读取失败");
    await waitText(page, created.name);
    assert.equal(await browser.execute(() => window.__uiAudit.unexpected.length), 0);
    assert.deepEqual(await snapshot(current.id, 1440, "write-result-kept", page, { createdId: created.id, refresh: "failed" }), []);
    current.visited.push(1440);
  });

  it("UX-1682 closes world editing after a committed save even when readback fails", async function () {
    current = { id: "UX-1682-world-save-read-failure", task: "世界观保存命令成功后读回失败仍应用回包内容并结束编辑", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("世界观");
    await waitText(page, "世界观设定");
    await browser.execute(() => {
      window.__uiAudit.fixtures.update_world_setting = { warnings: [] };
      window.__uiAudit.failNext.push({ command: "get_blueprint", status: 503, message: "世界观读回失败" });
    });
    await clickText("编辑世界观", page);
    const coreRules = await page.$("textarea");
    const savedText = "世界规则由保存命令回包确认，读回失败不应恢复编辑草稿。";
    await coreRules.setValue(savedText);
    await clickText("保存世界观", page);
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("世界观已保存"), { timeout });
    await waitText(page, "世界观读回失败");
    await waitText(page, savedText);
    assert.equal(await textButton("保存世界观", page).isExisting(), false);
    assert.equal(await textButton("编辑世界观", page).isExisting(), true);
    assert.deepEqual(await snapshot(current.id, 1440, "save-committed-read-error-visible", page, { savedText }), []);
    current.visited.push(1440);
  });

  it("UX-1683 and UX-1684 keep import queue and analysis progress read failures distinct", async function () {
    current = { id: "UX-1683-1684-import-read-errors", task: "导入队列和分析进度读取失败显示各自错误且不伪装空队列", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const queuePage = await project("导入/归纳", undefined, async () => {
      await browser.execute(() => window.__uiAudit.failNext.push(...Array.from({ length: 8 }, () => ({
        command: "list_import_queue_index", status: 503, message: "审核队列读取失败",
      }))));
    });
    await waitText(queuePage, "审核队列读取失败");
    assert.equal((await queuePage.getText()).includes("暂无待审核内容"), false);
    assert.deepEqual(await snapshot("UX-1683-import-queue-read-error", 1440, "queue-error-not-empty", queuePage), []);

    const progressPage = await project("导入/归纳", undefined, async () => {
      await browser.execute(() => window.__uiAudit.failNext.push(...Array.from({ length: 8 }, () => ({
        command: "get_project_import_analysis_progress", status: 503, message: "分析进度读取失败",
      }))));
    });
    await waitText(progressPage, "分析进度读取失败");
    assert.equal(await (await textButton("分析项目", progressPage)).isEnabled(), true);
    assert.equal((await progressPage.getText()).includes("暂无待审核内容"), false);
    assert.deepEqual(await snapshot("UX-1684-import-progress-read-error", 1440, "progress-error-not-idle", progressPage), []);
    current.visited.push(1440);
  });

  it("UX-2257 preserves queue rows but pauses their actions when queue refresh fails", async function () {
    current = { id: "UX-2257-import-queue-stale-read", task: "审核列表已显示时后台刷新失败，确认旧记录不再接受操作", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const filename = "01-旧书店与车站调查补充材料.md";
    const row = await page.$('.//li[.//button[@aria-label=' + literal(`查看 ${filename} 的审核详情`) + ']]');
    await row.waitForDisplayed({ timeout });
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "list_import_queue_index", status: 503, message: "审核队列刷新失败",
    }));
    await browser.waitUntil(async () => (await page.getText()).includes("审核队列刷新失败"), {
      timeout: 12000,
      timeoutMsg: "The five-second review queue poll must report its read failure",
    });
    assert.equal((await page.getText()).includes(filename), true);
    assert.equal(await (await row.$('button[aria-label=' + JSON.stringify(`选择 ${filename}`) + ']')).isEnabled(), false);
    assert.equal(await row.$('button[aria-haspopup="menu"]').isExisting(), false);
    assert.deepEqual(await snapshot(current.id, 1440, "old-row-visible-actions-paused", page, {
      rowPreservedForContext: true, actionDisabled: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2262 refreshes task state after a stale cancellation is rejected", async function () {
    current = { id: "UX-2262-import-cancel-conflict", task: "其他协作者先完成分析，本地停止命令失败后应更新进度与操作入口", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const progress = structuredClone(seed.commands.get_project_import_analysis_progress);
    progress.status = "analyzing";
    progress.message = "正在分析导入章节";
    progress.overall_progress = 45;
    progress.completed_at = null;
    progress.stages.generating_analysis_data.status = "in_progress";
    progress.stages.generating_analysis_data.completed = 5;
    const page = await project("导入/归纳", () => browser.execute((value) => {
      window.__uiAudit.fixtures.get_project_import_analysis_progress = value;
    }, progress));
    await textButton("停止分析", page).waitForDisplayed({ timeout });
    const progressReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project_import_analysis_progress").length,
    );
    const cancelRequests = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "cancel_import_analysis").length,
    );
    await browser.execute((completed) => {
      const audit = window.__uiAudit;
      audit.fixtures.get_project_import_analysis_progress = completed;
      audit.failNext.push({ command: "cancel_import_analysis", status: 409, message: "分析任务已完成" });
    }, structuredClone(seed.commands.get_project_import_analysis_progress));
    await press(await textButton("停止分析", page));
    await waitText($("body"), "分析任务已完成");
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project_import_analysis_progress").length > count,
    progressReads), { timeout });
    await browser.waitUntil(async () =>
      !(await textButton("停止分析", page).isDisplayed().catch(() => false)) &&
      await textButton("分析项目", page).isEnabled(),
    { timeout, timeoutMsg: "The refreshed completion state must remove the stale stop action" });
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "cancel_import_analysis").length,
    ), cancelRequests + 1);
    assert.deepEqual(await snapshot(current.id, 1440, "completed-state-restored", page, {
      rejectedCancellation: true,
      staleStopActionRemoved: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2263 rejects a second project analysis start and restores the running action", async function () {
    current = { id: "UX-2263-import-analysis-concurrent-start", task: "两个窗口同时启动同一项目分析，第二个请求收到运行中状态并收敛界面", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const running = structuredClone(seed.commands.get_project_import_analysis_progress);
    running.status = "analyzing";
    running.message = "正在其他窗口分析导入章节";
    running.overall_progress = 55;
    running.completed_at = null;
    running.stages.generating_analysis_data.status = "in_progress";
    running.stages.generating_analysis_data.completed = 6;
    const beforeRuns = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "run_import_project_analysis").length,
    );
    const beforeQueueReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_import_queue_index").length,
    );
    await browser.execute((progress) => {
      const audit = window.__uiAudit;
      audit.fixtures.get_project_import_analysis_progress = progress;
      audit.failNext.push({
        command: "run_import_project_analysis", status: 409,
        message: "该项目已有导入分析正在运行",
      });
    }, running);
    await clickText("分析项目", page);
    await waitText($("body"), "该项目已有导入分析正在运行");
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_import_queue_index").length > count,
    beforeQueueReads), { timeout });
    await textButton("停止分析", page).waitForDisplayed({ timeout });
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "run_import_project_analysis").length,
    ), beforeRuns + 1);
    assert.deepEqual(await snapshot(current.id, 1440, "running-task-action-restored", page, {
      duplicateStartRejected: true,
      activeTaskAction: "stop",
    }), []);
    current.visited.push(1440);
  });

  it("UX-2264 lets the server validate analysis start when progress cannot be read", async function () {
    current = { id: "UX-2264-import-analysis-progress-gate", task: "进度查询失败但项目可编辑时仍允许提交，由后端决定能否启动", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳", undefined, async () => {
      await browser.execute(() => window.__uiAudit.failNext.push(...Array.from({ length: 8 }, () => ({
        command: "get_project_import_analysis_progress", status: 503, message: "分析进度读取失败",
      }))));
    });
    await waitText(page, "分析进度读取失败");
    const start = await textButton("分析项目", page);
    assert.equal(await start.isEnabled(), true);
    const result = {
      analyzed_chapters: 12,
      progress: structuredClone(seed.commands.get_project_import_analysis_progress),
    };
    const beforeRuns = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "run_import_project_analysis").length,
    );
    await browser.execute((summary) => {
      window.__uiAudit.fixtures.run_import_project_analysis = summary;
    }, result);
    await press(start);
    await waitText($("body"), "已完成 12 章的分析与资料同步");
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "run_import_project_analysis").length,
    ), beforeRuns + 1);
    assert.deepEqual(await snapshot(current.id, 1440, "server-accepted-start", page, {
      initialProgressRead: "failed",
      startRequestSent: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2265 confirms permanent review deletion before calling the command", async function () {
    current = { id: "UX-2265-import-review-delete-confirm", task: "删除已解析审核记录前核对影响，取消不发删除命令", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const filename = "01-旧书店与车站调查补充材料.md";
    const row = await page.$('.//li[.//button[@aria-label=' + literal(`查看 ${filename} 的审核详情`) + ']]');
    const deleteCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "delete_import_review").length,
    );
    const openDelete = async () => {
      await press(await row.$('button[aria-haspopup="menu"]'));
      await menuItem("删除审核记录");
      return dialog("删除审核记录？");
    };
    let confirmation = await openDelete();
    await waitText(confirmation, "已接受的章节正文不会删除");
    await clickText("取消", confirmation);
    await confirmation.waitForExist({ reverse: true });
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "delete_import_review").length,
    ), deleteCount);
    assert.equal(await row.isExisting(), true);

    await browser.execute(() => { window.__uiAudit.fixtures.delete_import_review = null; });
    confirmation = await openDelete();
    await clickText("删除", confirmation);
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "delete_import_review").length === count + 1,
    deleteCount), { timeout });
    await browser.waitUntil(async () => !(await row.isExisting()), { timeout });
    await waitText($("body"), "已删除");
    assert.deepEqual(await snapshot(current.id, 1440, "permanent-delete-confirmed", page, {
      cancelSentNoDelete: true,
      confirmedDeleteRequestCount: 1,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2266 confirms terminal review rejection before changing its status", async function () {
    current = { id: "UX-2266-import-review-reject-confirm", task: "拒绝待审记录前确认不可逆状态，取消不发送命令", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const filename = "01-旧书店与车站调查补充材料.md";
    const row = await page.$('.//li[.//button[@aria-label=' + literal(`查看 ${filename} 的审核详情`) + ']]');
    const rejectCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "reject_import_review").length,
    );
    const openReject = async () => {
      await press(await row.$('button[aria-haspopup="menu"]'));
      await menuItem("拒绝审核");
      return dialog("拒绝审核项？");
    };
    let confirmation = await openReject();
    await waitText(confirmation, filename);
    await waitText(confirmation, "不能再从此队列接受");
    await clickText("取消", confirmation);
    await confirmation.waitForExist({ reverse: true });
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "reject_import_review").length,
    ), rejectCount);
    assert.equal(await row.isExisting(), true);

    const review = await browser.execute((name) => structuredClone(
      window.__uiAudit.fixtures.list_import_queue.find((item) => item.filename === name),
    ), filename);
    await browser.execute((item) => {
      const audit = window.__uiAudit;
      audit.fixtures.reject_import_review = { ...item, review_status: "rejected" };
      audit.fixtures.list_import_queue = audit.fixtures.list_import_queue.map((review) =>
        review.id === item.id ? { ...review, review_status: "rejected" } : review,
      );
    }, review);
    confirmation = await openReject();
    await clickText("拒绝审核", confirmation);
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "reject_import_review").length === count + 1,
    rejectCount), { timeout });
    await waitText(row, "已拒绝");
    await waitText($("body"), "已拒绝");
    assert.deepEqual(await snapshot(current.id, 1440, "rejected-after-confirmation", page, {
      cancelSentNoReject: true,
      confirmedRejectRequestCount: 1,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2267 refreshes the project list when deletion loses a race", async function () {
    current = { id: "UX-2267-project-delete-conflict", task: "其他协作者已删除项目，本地确认删除失败后同步项目列表", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    await home();
    const row = await rootRow();
    await row.waitForDisplayed({ timeout });
    const listReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_projects").length,
    );
    await press(await row.$('button[aria-label="更多操作"]'));
    await menuItem("删除项目");
    const confirmation = await dialog("删除项目");
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.fixtures.list_projects = {
        ...audit.fixtures.list_projects,
        items: audit.fixtures.list_projects.items.filter((project) => project.id !== 1),
        total: Math.max(0, audit.fixtures.list_projects.total - 1),
      };
      audit.fixtures.projectRecords = audit.fixtures.projectRecords.filter((project) => project.id !== 1);
      audit.failNext.push({ command: "delete_project", status: 404, message: "项目已被其他用户删除" });
    });
    await clickText("永久删除", confirmation);
    await waitText($("body"), "项目已被其他用户删除");
    await browser.waitUntil(async () => !(await row.isExisting()), { timeout });
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_projects").length > count,
    listReads), { timeout });
    assert.deepEqual(await snapshot(current.id, 1440, "externally-deleted-project-removed", $(".a4n-home-content"), {
      failedDeleteKeptError: true,
      refreshedListRemovedStaleProject: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2269 rejects a concurrent reverse generation for the same project target", async function () {
    current = { id: "UX-2269-import-reverse-concurrent-run", task: "另一窗口正在生成同一项目章纲候选时拒绝重复模型调用", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const summary = await containing("项目资料归纳", page, "summary");
    await press(summary);
    const tools = await summary.$("..");
    await press(await tools.$('button[role="combobox"]'));
    await press(await $('//*[@role="option" and normalize-space(.)=' + literal("章节大纲") + ']'));
    const beforeRequests = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "reverse_engineer").length,
    );
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "reverse_engineer", status: 409,
      message: "状态无效：当前状态 该项目的同类工作流正在运行，不允许操作 再次启动该工作流",
    }));
    await clickText("生成章节大纲候选", tools);
    await waitText($("body"), "该项目的同类工作流正在运行");
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "reverse_engineer").length,
    ), beforeRequests + 1);
    assert.equal((await tools.getText()).includes("确认更新"), false);
    assert.deepEqual(await snapshot(current.id, 1440, "concurrent-target-not-overwritten", page, {
      rejectedConcurrentRun: true,
      candidateNotPresentedForRejectedRequest: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2270 removes a reverse candidate after its source context changes", async function () {
    current = { id: "UX-2270-import-reverse-stale-candidate", task: "项目上下文改变后确认旧章纲候选失败，应同步掉过期候选", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const summary = await containing("项目资料归纳", page, "summary");
    await press(summary);
    const tools = await summary.$("..");
    const proposal = {
      workflow_run_id: 991,
      target: "outline",
      state: "proposal",
      warnings: [],
      result: { outlines: [{ chapter_number: 1, title: "钟楼来信", summary: "第一声钟响后收到来信。" }] },
      applied: null,
    };
    await browser.execute((value) => {
      window.__uiAudit.fixtures.get_reverse_engineering_candidate = value;
    }, proposal);
    await press(await tools.$('button[role="combobox"]'));
    await press(await $('//*[@role="option" and normalize-space(.)=' + literal("章节大纲") + ']'));
    await waitText(tools, "确认更新章节大纲");

    const readsBeforeConfirmation = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_reverse_engineering_candidate").length,
    );
    const confirmationsBefore = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "confirm_reverse_engineering_candidate").length,
    );
    await clickText("确认更新章节大纲", tools);
    const confirmation = await dialog("确认更新章节大纲");
    await browser.execute(() => {
      window.__uiAudit.fixtures.get_reverse_engineering_candidate = null;
      window.__uiAudit.failNext.push({
        command: "confirm_reverse_engineering_candidate",
        status: 409,
        message: "项目上下文已变化，请重新归纳",
      });
    });
    await clickText("更新章节大纲", confirmation);
    await waitText($("body"), "项目上下文已变化，请重新归纳");
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_reverse_engineering_candidate").length > count,
    readsBeforeConfirmation), { timeout, timeoutMsg: "The rejected candidate must be reconciled with the current source state" });
    await browser.waitUntil(async () => !(await tools.getText()).includes("确认更新"), {
      timeout,
      timeoutMsg: "The source-invalid candidate must no longer be confirmable",
    });
    const confirmationRequests = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "confirm_reverse_engineering_candidate").length,
    );
    assert.equal(confirmationRequests, confirmationsBefore + 1);
    assert.deepEqual(await snapshot(current.id, 1440, "stale-candidate-removed", page, {
      freshnessConflictShown: true,
      candidateReadBackAfterConflict: true,
      staleConfirmationButtonRemoved: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2274 keeps the current reverse candidate when a new run is rejected", async function () {
    current = { id: "UX-2274-import-reverse-preserve-candidate", task: "反推失败或被并发门禁拒绝后仍保留原可确认候选", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const summary = await containing("项目资料归纳", page, "summary");
    await press(summary);
    const tools = await summary.$("..");
    const proposal = {
      workflow_run_id: 992,
      target: "outline",
      state: "proposal",
      warnings: [],
      result: { outlines: [{ chapter_number: 13, title: "候选应保留", summary: "这是已经生成的候选。" }] },
      applied: null,
    };
    await browser.execute((value) => {
      window.__uiAudit.fixtures.get_reverse_engineering_candidate = value;
      window.__uiAudit.failNext.push({
        command: "reverse_engineer",
        status: 409,
        message: "状态无效：当前状态 该项目的同类工作流正在运行，不允许操作 再次启动该工作流",
      });
    }, proposal);
    await press(await tools.$('button[role="combobox"]'));
    await press(await $('//*[@role="option" and normalize-space(.)=' + literal("章节大纲") + ']'));
    await waitText(tools, "确认更新章节大纲");
    const beforeRuns = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "reverse_engineer").length,
    );

    await clickText("生成章节大纲候选", tools);
    await waitText($("body"), "该项目的同类工作流正在运行");
    await waitText(tools, "查看候选内容");
    const confirm = await textButton("确认更新章节大纲", tools);
    assert.equal(await confirm.isEnabled(), true);
    const runsAfterFailure = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "reverse_engineer").length,
    );
    assert.equal(runsAfterFailure, beforeRuns + 1);
    assert.deepEqual(await snapshot(current.id, 1440, "candidate-survives-rejected-regeneration", page, {
      startRejectedByRunningWorkflow: true,
      previousCandidateVisible: true,
      previousCandidateStillConfirmable: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-2273 rejects a second outline run for the same project workflow", async function () {
    current = { id: "UX-2273-outline-concurrent-run", task: "另一窗口正在生成同项目章纲时拒绝第二个 workflow run", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("大纲");
    await clickText("章节大纲", page);
    const summary = await containing("追加章纲", page, "summary");
    await press(summary);
    const section = await summary.$("..");
    await section.$('input[type="number"]').setValue("2");
    const beforeRuns = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "start_outline_generation").length,
    );
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "start_outline_generation",
      status: 409,
      message: "状态无效：当前状态 该项目的同类工作流正在运行，不允许操作 再次启动该工作流",
    }));

    await clickText("追加章纲", section);
    await waitText($("body"), "该项目的同类工作流正在运行");
    const runsAfterConflict = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "start_outline_generation").length,
    );
    assert.equal(runsAfterConflict, beforeRuns + 1);
    assert.deepEqual(await snapshot(current.id, 1440, "duplicate-outline-run-rejected", page, {
      duplicateRunRejected: true,
      runningWorkflowFeedback: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1685 keeps imported review rows and success feedback when queue readback fails", async function () {
    current = { id: "UX-1685-import-upload-read-failure", task: "导入命令返回新审核项后列表读取失败仍呈现回包实体和成功结果", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const created = {
      ...structuredClone(seed.commands.list_import_queue[0]),
      id: 599,
      filename: "界面审查导入的新审核项.md",
      project_id: 1,
      review_status: "pending",
      created_at: "2026-10-10T00:00:00Z",
      updated_at: "2026-10-10T00:00:00Z",
    };
    await browser.execute(({ review }) => {
      window.__uiAudit.fixtures.import_project_text_upload = [review];
      window.__uiAudit.failNext.push({ command: "list_import_queue_index", status: 503, message: "导入后的审核队列读取失败" });
      const input = document.querySelector('input[type="file"]');
      const transfer = new DataTransfer();
      transfer.items.add(new File(["界面审查导入内容"], "界面审查导入.md", { type: "text/markdown" }));
      Object.defineProperty(input, "files", { configurable: true, value: transfer.files });
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }, { review: created });
    await waitText(page, "界面审查导入.md");
    await clickText("开始导入", page);
    await browser.waitUntil(() => browser.execute(() =>
      [...document.querySelectorAll(".a4n-toast")].some((toast) => toast.textContent?.includes("已创建 1 个审核项")),
    ), { timeout });
    await waitText(page, "导入后的审核队列读取失败");
    await waitText(page, created.filename);
    await waitText(page, "尚未选择文件");
    assert.deepEqual(await snapshot(current.id, 1440, "import-result-kept", page, { createdReviewId: created.id, refresh: "failed" }), []);
    current.visited.push(1440);
  });

  it("UX-1686 keeps an analysis completion result when queue readback fails", async function () {
    current = { id: "UX-1686-import-analysis-read-failure", task: "导入分析命令返回完成后队列读取失败不改写为任务失败", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const result = {
      analyzed_chapters: 12,
      progress: structuredClone(seed.commands.get_project_import_analysis_progress),
    };
    await browser.execute((summary) => {
      window.__uiAudit.fixtures.run_import_project_analysis = summary;
      window.__uiAudit.failNext.push({ command: "list_import_queue_index", status: 503, message: "分析后的审核队列读取失败" });
    }, result);
    await clickText("分析项目", page);
    await waitText(page, "分析后的审核队列读取失败");
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("已完成 12 章的分析与资料同步"), { timeout });
    await waitText(page, "已完成");
    assert.equal((await page.getText()).includes("导入项目分析失败"), false);
    assert.deepEqual(await snapshot(current.id, 1440, "analysis-result-kept", page, { analyzedChapters: result.analyzed_chapters, queueRead: "failed" }), []);
    current.visited.push(1440);
  });

  it("UX-2258 reloads review state after a stale action is rejected", async function () {
    current = { id: "UX-2258-import-action-conflict", task: "其他协作者先处理审核项后，本地接受命令失败并同步最新状态", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const filename = "01-旧书店与车站调查补充材料.md";
    const reviewButton = await page.$('button[aria-label=' + JSON.stringify(`查看 ${filename} 的审核详情`) + ']');
    const previousReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_import_queue_index").length,
    );
    await browser.execute((name) => {
      const audit = window.__uiAudit;
      audit.fixtures.accept_import_review = null;
      audit.fixtures.list_import_queue = audit.fixtures.list_import_queue.map((review) =>
        review.filename === name ? { ...review, review_status: "accepted" } : review,
      );
      audit.failNext.push({
        command: "accept_import_review", status: 409, message: "该审核项已被其他协作者处理",
      });
    }, filename);
    await press((await page.$$('ul button[aria-haspopup="menu"]'))[0]);
    await menuItem("接受审核");
    await waitText($("body"), "该审核项已被其他协作者处理");
    await waitText(reviewButton, "已接受");
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_import_queue_index").length > count,
    previousReads), { timeout });
    assert.equal(await reviewButton.isEnabled(), true);
    assert.deepEqual(await snapshot(current.id, 1440, "server-review-state-restored", page, {
      rejectedStaleCommand: true, currentStatus: "accepted",
    }), []);
    current.visited.push(1440);
  });

  it("UX-2261 reconciles a partial batch when another reviewer already accepted one item", async function () {
    current = { id: "UX-2261-import-batch-conflict", task: "批量接受中一项已被其他协作者处理，确认部分成功和冲突项均同步", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const firstName = "01-旧书店与车站调查补充材料.md";
    const secondName = "02-旧书店与车站调查补充材料.md";
    const [first, second] = await browser.execute(({ firstName, secondName }) => {
      const rows = window.__uiAudit.fixtures.list_import_queue;
      return structuredClone([
        rows.find((review) => review.filename === firstName),
        rows.find((review) => review.filename === secondName),
      ]);
    }, { firstName, secondName });
    assert.ok(first && second, "The queue fixture must contain both selected reviews");
    const acceptedByBatch = { ...second, review_status: "accepted" };
    const previousQueueReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_import_queue_index").length,
    );
    await browser.execute(({ first, second, acceptedByBatch }) => {
      const audit = window.__uiAudit;
      audit.fixtures.batch_accept_reviews = {
        accepted_reviews: [acceptedByBatch],
        failure: { review_id: first.id, message: "该审核项已被其他协作者接受" },
      };
      audit.fixtures.list_import_queue = audit.fixtures.list_import_queue.map((review) =>
        review.id === first.id || review.id === second.id
          ? { ...review, review_status: "accepted" }
          : review,
      );
    }, { first, second, acceptedByBatch });
    await press(await ariaButton(`选择 ${firstName}`, page));
    await press(await ariaButton(`选择 ${secondName}`, page));
    await clickText("接受所选（2）", page);
    await waitText($("body"), "该审核项已被其他协作者接受");
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_import_queue_index").length > count,
    previousQueueReads), { timeout });
    await waitText(await ariaButton(`查看 ${firstName} 的审核详情`, page), "已接受");
    await waitText(await ariaButton(`查看 ${secondName} 的审核详情`, page), "已接受");
    assert.deepEqual(await snapshot(current.id, 1440, "batch-conflict-reconciled", page, {
      batchAcceptedId: second.id,
      concurrentlyAcceptedId: first.id,
    }), []);
    current.visited.push(1440);
  });

  it("UX-1688 applies batch review command results before queue readback", async function () {
    current = { id: "UX-1688-batch-accept-read-failure", task: "批量接受实体回包更新各行并清除所选，即使后续队列读取失败", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const accepted = seed.commands.list_import_queue.slice(0, 2).map((review) => ({ ...review, review_status: "accepted" }));
    await browser.execute((result) => {
      window.__uiAudit.fixtures.batch_accept_reviews = { accepted_reviews: result, failure: null };
      window.__uiAudit.failNext.push({ command: "list_import_queue_index", status: 503, message: "批量接受后的审核队列读取失败" });
    }, accepted);
    await press(await ariaButton("选择 01-旧书店与车站调查补充材料.md", page));
    await press(await ariaButton("选择 02-旧书店与车站调查补充材料.md", page));
    await clickText("接受所选（2）", page);
    await browser.waitUntil(() => browser.execute(() =>
      [...document.querySelectorAll(".a4n-toast")].some((toast) => toast.textContent?.includes("已批量接受 2 个")),
    ), { timeout });
    await waitText(page, "批量接受后的审核队列读取失败");
    for (const filename of accepted.map((review) => review.filename)) {
      const detailButton = await page.$('button[aria-label=' + JSON.stringify(`查看 ${filename} 的审核详情`) + ']');
      await waitText(detailButton, "已接受");
      assert.equal(await (await page.$('button[aria-label=' + JSON.stringify(`选择 ${filename}`) + ']')).getAttribute("aria-pressed"), "false");
    }
    assert.deepEqual(await snapshot(current.id, 1440, "batch-result-kept", page, { acceptedIds: accepted.map((review) => review.id) }), []);
    current.visited.push(1440);
  });

  it("UX-1784 keeps a partial batch-accept result aligned with the server queue", async function () {
    current = { id: "UX-1784-batch-accept-partial", task: "批量接受中途失败时同步已完成项目并保留未完成项状态", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const [first, second, ...remaining] = seed.commands.list_import_queue;
    const acceptedFirst = { ...first, review_status: "accepted" };
    const queueAfterOtherWindowChange = [acceptedFirst, ...remaining];
    await browser.execute(({ accepted, failed, nextQueue }) => {
      const fixtures = window.__uiAudit.fixtures;
      fixtures.batch_accept_reviews = {
        accepted_reviews: [accepted],
        failure: { review_id: failed.id, message: "该审核项已在另一个窗口中删除" },
      };
      fixtures.list_import_queue = nextQueue;
    }, { accepted: acceptedFirst, failed: second, nextQueue: queueAfterOtherWindowChange });
    await press(await ariaButton(`选择 ${first.filename}`, page));
    await press(await ariaButton(`选择 ${second.filename}`, page));
    const readsBefore = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "list_import_queue_index",
    ).length);

    await clickText("接受所选（2）", page);
    await waitText($("body"), `已接受 1 个审核项；${second.filename} 未完成`);
    const acceptedDetail = await page.$(`button[aria-label=${JSON.stringify(`查看 ${first.filename} 的审核详情`)}]`);
    await waitText(acceptedDetail, "已接受");
    assert.equal(await (await page.$(`button[aria-label=${JSON.stringify(`选择 ${first.filename}`)}]`)).getAttribute("aria-pressed"), "false");
    assert.equal(await page.$(`button[aria-label=${JSON.stringify(`选择 ${second.filename}`)}]`).isExisting(), false);
    const readsAfter = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "list_import_queue_index",
    ).length);
    assert.equal(readsAfter, readsBefore + 1, "Partial results must refresh the authoritative queue once");
    current.evidence = { acceptedReviewId: first.id, failedReviewId: second.id, queueReadback: true, remainingPendingItems: remaining.length };
    current.visited.push(1440);
  });

  it("UX-1785 uses the import response as the new chapter save result", async function () {
    current = { id: "UX-1785-new-chapter-save-result", task: "新建章节保存直接消费写回包，不因重复索引读取失败误报", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    await writing();
    await press(await $('button[aria-label="新建章节"]'));
    const prompt = await dialog("新建章节");
    await prompt.$("input").setValue("10");
    await clickText("进入章节", prompt);
    const editor = await $('textarea[aria-label="章节正文"]');
    await browser.waitUntil(async () => (await editor.getValue()) === "", { timeout });
    const savedContent = "新建章节保存响应中的正文。";
    await editor.setValue(savedContent);
    const imported = {
      ...seed.commands.list_chapters[0],
      id: 110,
      chapter_number: 10,
      title: "第10章 · 新线索",
      content: savedContent,
      edit_revision: "ux-1785-revision",
      derived_sync_status: "current",
    };
    await browser.execute((chapter) => {
      window.__uiAudit.fixtures.import_chapter = chapter;
      window.__uiAudit.fixtures.list_chapters = [
        ...window.__uiAudit.fixtures.list_chapters.filter((item) => item.id !== chapter.id),
        chapter,
      ];
      window.__uiAudit.failNext.push({
        command: "list_chapter_index", status: 503, message: "章节索引回读失败注入",
      });
    }, imported);
    const indexReadsBefore = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "list_chapter_index",
    ).length);

    await clickText("保存修改");
    await waitText($("body"), "章节正文与派生资料已保存");
    assert.equal(await editor.getValue(), savedContent);
    const indexReadsAfter = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "list_chapter_index",
    ).length);
    assert.equal(indexReadsAfter, indexReadsBefore, "The import response is sufficient to project the saved chapter");
    assert.equal(await browser.execute(() => window.__uiAudit.failNext.some((item) => item.command === "list_chapter_index")), true);
    await browser.execute(() => {
      window.__uiAudit.failNext = window.__uiAudit.failNext.filter((item) => item.command !== "list_chapter_index");
    });
    current.evidence = { importChapterResponse: imported.id, chapterNumber: 10, appliedToEditor: true, redundantRead: false };
    current.visited.push(1440);
  });

  it("UX-1786 keeps chapter save success separate from local draft cleanup failure", async function () {
    current = { id: "UX-1786-chapter-save-draft-cleanup", task: "正文保存成功而本地草稿删除失败时明确显示两种结果", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    await writing();
    const editor = await $('textarea[aria-label="章节正文"]');
    const savedContent = seed.commands.list_chapters[0].content + " 本地草稿清理失败用例。";
    const draftKey = "a4n:writing_draft:1:1";
    const updatedChapter = {
      ...seed.commands.list_chapters[0],
      content: savedContent,
      edit_revision: "ux-1786-revision",
    };
    await browser.execute(({ key, chapter }) => {
      localStorage.setItem(key, JSON.stringify({ content: chapter.content, updatedAt: new Date().toISOString() }));
      window.__uiAudit.fixtures.update_chapter_content = chapter;
      window.__uiAudit.originalStorageRemoveItem = Storage.prototype.removeItem;
      Storage.prototype.removeItem = function (candidate) {
        if (candidate === key) throw new Error("本地草稿删除失败");
        return window.__uiAudit.originalStorageRemoveItem.call(this, candidate);
      };
    }, { key: draftKey, chapter: updatedChapter });

    try {
      await editor.setValue(savedContent);
      await clickText("保存修改");
      await waitText($("body"), "本地草稿清理失败");
      await browser.waitUntil(() => browser.execute(() => [...document.querySelectorAll('[role="status"].a4n-toast')]
        .some((toast) => (toast.textContent ?? "").includes("正文已保存")
          && (toast.textContent ?? "").includes("本地草稿清理失败"))), { timeout });
      assert.equal(await editor.getValue(), savedContent);
      assert.equal(await $('.a4n-writing-save-state').getAttribute("data-state"), "saved");
      assert.equal(await browser.execute((key) => localStorage.getItem(key) != null, draftKey), true);
      current.evidence = { updateChapterResponse: true, localCleanupWarning: true, editorState: "saved", draftRemained: true };
      current.visited.push(1440);
    } finally {
      await browser.execute((key) => {
        const original = window.__uiAudit.originalStorageRemoveItem;
        if (!original) return;
        Storage.prototype.removeItem = original;
        delete window.__uiAudit.originalStorageRemoveItem;
        original.call(localStorage, key);
      }, draftKey);
    }
  });

  it("UX-1787 keeps shooting-script deletion successful when the delete command commits", async function () {
    current = { id: "UX-1787-shooting-script-delete-readback", task: "拍摄剧本删除命令成功后不因额外读取失败误报", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const page = await videoScreenplay();
    const scriptReadsBefore = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "get_chapter_shooting_script",
    ).length);
    await browser.execute(() => {
      window.__uiAudit.fixtures.delete_chapter_shooting_script = null;
      window.__uiAudit.failNext.push({
        command: "get_chapter_shooting_script", status: 503, message: "删除后的拍摄剧本读取失败",
      });
    });
    await clickText("删除拍摄剧本", page);
    const confirmation = await dialog("删除拍摄剧本");
    await clickText("删除拍摄剧本", confirmation);
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("拍摄剧本已删除"), { timeout });
    await waitText(page, "当前章节尚未生成拍摄剧本");
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "get_chapter_shooting_script",
    ).length), scriptReadsBefore, "A committed delete must not depend on another script read");
    await browser.execute(() => {
      window.__uiAudit.failNext = window.__uiAudit.failNext.filter(
        (item) => item.command !== "get_chapter_shooting_script",
      );
    });
    current.evidence = { deleteCommandCommitted: true, scriptRemovedFromView: true, redundantRead: false };
    current.visited.push(1440);
  });

  it("UX-1788 shows partial shooting-script progress when no complete script is saved", async function () {
    current = { id: "UX-1788-shooting-script-partial-read", task: "正式剧本尚未写入但部分场景进度可用时仍显示可恢复场景", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const page = await videoScreenplay();
    const script = structuredClone(mediaSeed.byChapter[1].get_chapter_shooting_script.script);
    await browser.execute((progress) => {
      window.__uiAudit.fixtures.byChapter[1].get_chapter_shooting_script = null;
      window.__uiAudit.fixtures.byProject[3].byChapter[1].get_chapter_shooting_script = null;
      window.__uiAudit.fixtures.byChapter[1].get_chapter_shooting_script_unit_progress = progress;
      window.__uiAudit.fixtures.byProject[3].byChapter[1].get_chapter_shooting_script_unit_progress = progress;
    }, { completed_unit_count: 1, total_unit_count: 6, workflow_run_id: 9901, script });
    await press(await $('//*[@role="tab" and contains(normalize-space(.),"导演编排")]'));
    await press(await $('//*[@role="tab" and contains(normalize-space(.),"拍摄剧本")]'));
    await waitText(page, "已完成 1/6 个拍摄场景");
    current.evidence = { completeScriptRead: "failed", partialProgressRead: "successful", partialProgressVisible: true };
    current.visited.push(1440);
  });

  it("UX-1789 keeps the visual-plan stage actionable after re-entry", async function () {
    current = { id: "UX-1789-shooting-script-plan-retry", task: "重新进入视频项目拍摄剧本阶段后保留同章节重生成入口", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const page = await videoScreenplay();
    await press(await $('//*[@role="tab" and contains(normalize-space(.),"导演编排")]'));
    await press(await $('//*[@role="tab" and contains(normalize-space(.),"拍摄剧本")]'));
    const regenerate = await textButton("重新生成拍摄剧本", page);
    assert.equal(await regenerate.isEnabled(), true);
    current.evidence = { stageReentered: true, sameChapterScope: true, generationEnabled: true };
    current.visited.push(1440);
  });

  it("UX-1790 keeps chapter analysis success when chapter readback fails", async function () {
    current = { id: "UX-1790-chapter-analysis-readback", task: "章节分析命令回包成功后不因章节索引或大纲读取失败误报", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const page = await writing("分析");
    const source = structuredClone(seed.commands.list_chapters[0]);
    const analyzed = {
      ...source,
      analysis_data: {
        ...source.analysis_data,
        summaries: {
          ...source.analysis_data.summaries,
          compressed: "分析命令回包已保存的新摘要。",
        },
      },
      analysis_sync_status: "completed",
    };
    await browser.execute((chapter) => {
      window.__uiAudit.fixtures.generate_chapter_analysis = { chapter, warnings: [] };
      window.__uiAudit.failNext.push(
        { command: "list_chapter_index", status: 503, message: "分析后的章节索引读取失败" },
        { command: "list_chapter_outline_index", status: 503, message: "分析后的大纲读取失败" },
      );
    }, analyzed);
    await clickText("重新分析", page);
    await waitText($("body"), "章节结构化分析");
    await waitText($("body"), "任务已完成");
    await waitText($("body"), "分析后的章节索引读取失败");
    await waitText($("body"), "分析后的大纲读取失败");
    const analysisProgress = await dialog("章节结构化分析");
    await clickText("关闭", analysisProgress);
    assert.equal((await $(".a4n-toast").getText()).includes("结构化分析失败"), false);
    assert.equal((await $(".a4n-toast").getText()).includes("结构化分析已保存"), true);
    current.evidence = { mutationCommitted: true, chapterReadback: "failed", outlineReadback: "failed", savedResultVisible: true };
    current.visited.push(1440);
  });

  it("UX-1691 applies a modified review result and success feedback before queue readback", async function () {
    current = { id: "UX-1691-modify-review-read-failure", task: "审核修改接受命令回包更新详情并提示成功，不因队列读取失败回滚", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const source = structuredClone(seed.commands.list_import_queue[0]);
    const modified = { ...source, review_status: "modified", analysis_result: { ...source.analysis_result, summary: "修改后的审核结果由命令回包确认。" } };
    await browser.execute((result) => {
      window.__uiAudit.fixtures.accept_import_review = result;
      window.__uiAudit.failNext.push({ command: "list_import_queue_index", status: 503, message: "修改后的审核队列读取失败" });
    }, modified);
    await press(await page.$('button[aria-label="查看 01-旧书店与车站调查补充材料.md 的审核详情"]'));
    const aside = await page.$("aside");
    await press(await aside.$("summary"));
    const editor = await aside.$("textarea");
    const edited = { ...JSON.parse(await editor.getValue()), summary: "作者确认的修改结果。" };
    await editor.setValue(JSON.stringify(edited, null, 2));
    await clickText("保存并接受", aside);
    await browser.waitUntil(() => browser.execute(() =>
      [...document.querySelectorAll(".a4n-toast")].some((toast) => toast.textContent?.includes("审核内容已更新")),
    ), { timeout });
    await waitText(page, "修改后的审核队列读取失败");
    await waitText(aside, "已修改");
    assert.equal((await aside.$("textarea").getValue()).includes("修改后的审核结果由命令回包确认。"), true);
    assert.deepEqual(await snapshot(current.id, 1440, "modified-result-kept", page, { reviewId: modified.id, refresh: "failed" }), []);
    current.visited.push(1440);
  });

  it("UX-1687 projects cancellation state from the command and permits analysis to run again", async function () {
    current = { id: "UX-1687-import-analysis-cancel-contract", task: "取消分析以命令返回的已持久化状态更新页面并释放下次运行", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const running = structuredClone(seed.commands.get_project_import_analysis_progress);
    Object.assign(running, {
      cancelled_at: null,
      completed_at: null,
      failed_at: null,
      message: "正在分析第8份材料，已完成7/12。",
      overall_progress: 58,
      status: "analyzing",
    });
    Object.assign(running.stages.generating_analysis_data, { completed: 7, status: "in_progress" });
    const cancelled = {
      ...running,
      cancelled_at: "2026-10-10T00:00:01Z",
      message: "导入分析已取消",
      status: "cancelled",
    };
    const completed = {
      ...running,
      completed_at: "2026-10-10T00:00:02Z",
      cancelled_at: null,
      message: "导入分析完成",
      overall_progress: 100,
      status: "completed",
      stages: { generating_analysis_data: { ...running.stages.generating_analysis_data, completed: 12, status: "completed" } },
    };
    const page = await project("导入/归纳", async () => {
      await browser.execute(({ runningProgress, cancelledProgress, completedProgress }) => {
        const fixtures = window.__uiAudit.fixtures;
        fixtures.get_project_import_analysis_progress = runningProgress;
        fixtures.cancel_import_analysis = cancelledProgress;
        fixtures.run_import_project_analysis = { analyzed_chapters: 12, progress: completedProgress };
      }, { runningProgress: running, cancelledProgress: cancelled, completedProgress: completed });
    });
    await clickText("停止分析", page);
    const cancelledDialog = await dialog("导入项目分析");
    await waitText(cancelledDialog, "任务已停止");
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("导入项目分析已取消"), { timeout });
    const cancelRequest = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "cancel_import_analysis"));
    assert.equal(cancelRequest.body.projectId, 1);
    await clickText("关闭", cancelledDialog);
    await clickText("继续分析", page);
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "run_import_project_analysis").length === 1), { timeout });
    const completedDialog = await dialog("导入项目分析");
    await waitText(completedDialog, "任务已完成");
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("已完成 12 章的分析与资料同步"), { timeout });
    assert.deepEqual(await browser.execute(() => ({
      cancelCount: window.__uiAudit.requests.filter((item) => item.command === "cancel_import_analysis").length,
      runCount: window.__uiAudit.requests.filter((item) => item.command === "run_import_project_analysis").length,
      unexpected: window.__uiAudit.unexpected,
    })), { cancelCount: 1, runCount: 1, unexpected: [] });
    await clickText("关闭", completedDialog);
    assert.deepEqual(await snapshot(current.id, 1440, "cancelled-then-rerun", page, { cancelResult: cancelled.status, rerunResult: completed.status }), []);
    current.visited.push(1440);
  });

  it("UX-1689 preserves a successful review deletion when its queue refresh fails", async function () {
    current = { id: "UX-1689-review-delete-read-failure", task: "删除审核项成功后队列读取失败仍立即移除对应列表行并反馈成功", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    await waitText(page, "01-旧书店与车站调查补充材料.md");
    const reviewButton = (await page.$$('button[aria-label*="的审核详情"]'))[0];
    const reviewLabel = await reviewButton.getAttribute("aria-label");
    await press((await page.$$('ul button[aria-haspopup="menu"]'))[0]);
    await browser.execute(() => {
      window.__uiAudit.fixtures.delete_import_review = null;
      window.__uiAudit.failNext.push(...Array.from({ length: 8 }, () => ({
        command: "list_import_queue_index", status: 503, message: "删除后的审核队列读取失败",
      })));
    });
    await menuItem("删除审核记录");
    const deleteConfirmation = await dialog("删除审核记录？");
    await clickText("删除", deleteConfirmation);
    await waitText($("body"), "已删除");
    await waitText(page, "删除后的审核队列读取失败");
    assert.equal((await page.getText()).includes("01-旧书店与车站调查补充材料.md"), false);
    assert.deepEqual(await snapshot(current.id, 1440, "delete-result-kept", page, { deletedReviewLabel: reviewLabel }), []);
    current.visited.push(1440);
  });

  it("UX-1690 and UX-1694 require draft disposition on review switches and preserve it when its source is deleted", async function () {
    current = { id: "UX-1690-1694-import-review-draft-ownership", task: "切换审核先确认草稿处置；外部删除来源后保留 JSON 草稿", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const firstReview = await page.$('button[aria-label="查看 01-旧书店与车站调查补充材料.md 的审核详情"]');
    const secondReview = await page.$('button[aria-label="查看 02-旧书店与车站调查补充材料.md 的审核详情"]');
    await press(firstReview);
    const aside = await page.$("aside");
    const jsonEditor = await openImportReviewEditor(aside);
    const original = JSON.parse(await jsonEditor.getValue());
    const draft = { ...original, summary: original.summary + " 本地草稿保留。" };
    const draftText = JSON.stringify(draft, null, 2);
    assert.equal(await jsonEditor.isEnabled(), true);
    assert.equal((await wheelTo(jsonEditor)).reached, true);
    await jsonEditor.setValue(draftText);

    await press(await ariaButton("选择 01-旧书店与车站调查补充材料.md", page));
    await clickText("接受所选（1）", page);
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("请先保存或放弃当前审核项的 JSON 修改"), { timeout });
    await press((await page.$$('ul button[aria-haspopup="menu"]'))[0]);
    await menuItem("接受审核");
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("请先保存或放弃当前审核项的 JSON 修改"), { timeout });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => ["batch_accept_reviews", "accept_import_review"].includes(item.command))), false);

    await press(secondReview);
    const switchGuard = await dialog("未保存修改");
    await clickText("取消", switchGuard);
    await switchGuard.waitForExist({ reverse: true });
    assert.equal(await jsonEditor.getValue(), draftText);
    await press(secondReview);
    const discardGuard = await dialog("未保存修改");
    await clickText("放弃草稿", discardGuard);
    await discardGuard.waitForExist({ reverse: true });
    await waitText(aside, "02-旧书店与车站调查补充材料.md");
    await press(firstReview);
    await waitText(aside, "01-旧书店与车站调查补充材料.md");
    const firstEditor = await openImportReviewEditor(aside);
    const firstSource = JSON.parse(await firstEditor.getValue());
    const preservedDraft = { ...firstSource, summary: firstSource.summary + " 来源被其他窗口删除后仍保留。" };
    const preservedText = JSON.stringify(preservedDraft, null, 2);
    assert.equal(await firstEditor.isEnabled(), true);
    await firstEditor.setValue(preservedText);
    await browser.execute((reviewId) => {
      window.__uiAudit.fixtures.list_import_queue = window.__uiAudit.fixtures.list_import_queue.filter((review) => review.id !== reviewId);
    }, 501);
    await browser.waitUntil(async () => (await aside.getText()).includes("此审核记录已从队列删除"), { timeout: 12000 });
    assert.equal(await firstEditor.getValue(), preservedText);
    assert.equal(await firstEditor.isEnabled(), true);
    assert.equal(await (await textButton("保存并接受", aside)).isEnabled(), false);
    await clickText("放弃草稿", aside);
    await browser.waitUntil(async () => (await aside.getText()).trim() === "", { timeout });
    assert.deepEqual(await snapshot(current.id, 1440, "deleted-source-draft-preserved", page, { draftVisibleUntilExplicitDiscard: true }), []);
    current.visited.push(1440);
  });

  it("UX-2260 keeps discard available when another reviewer changes a dirty review", async function () {
    current = { id: "UX-2260-import-review-status-draft", task: "他人接受审核项后，本地 JSON 草稿保持可见并能显式放弃", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("导入/归纳");
    const filename = "01-旧书店与车站调查补充材料.md";
    const reviewButton = await page.$('button[aria-label=' + JSON.stringify(`查看 ${filename} 的审核详情`) + ']');
    await press(reviewButton);
    const aside = await page.$("aside");
    const editor = await openImportReviewEditor(aside);
    const initial = JSON.parse(await editor.getValue());
    const draft = JSON.stringify({ ...initial, summary: initial.summary + " 本地待放弃草稿。" }, null, 2);
    await editor.setValue(draft);
    await browser.execute((name) => {
      window.__uiAudit.fixtures.list_import_queue = window.__uiAudit.fixtures.list_import_queue.map((review) =>
        review.filename === name ? { ...review, review_status: "accepted" } : review,
      );
    }, filename);
    await waitText(aside, "审核项状态已变化");
    assert.equal(await editor.getValue(), draft);
    assert.equal(await editor.isEnabled(), false);
    assert.equal(await (await textButton("保存并接受", aside)).isEnabled(), false);
    const discard = await textButton("放弃草稿", aside);
    assert.equal(await discard.isEnabled(), true);
    await press(discard);
    await browser.waitUntil(async () => (await aside.getText()).trim() === "", { timeout });
    assert.deepEqual(await snapshot(current.id, 1440, "changed-review-draft-discardable", page, {
      draftPreservedUntilExplicitDiscard: true,
      discardEnabledAfterStatusChange: true,
    }), []);
    current.visited.push(1440);
  });

  it("UX-B74 preserves chapter/count drafts and rejects invalid ranges", async function () {
    const requestCount = (command) => browser.execute((name) =>
      window.__uiAudit.requests.filter((item) => item.command === name).length, command);
    const latestRequest = (command) => browser.execute((name) =>
      window.__uiAudit.requests.findLast((item) => item.command === name)?.body.input, command);
    const waitForRequest = async (command, previousCount) => browser.waitUntil(
      async () => (await requestCount(command)) > previousCount,
      { timeout, timeoutMsg: "Expected request: " + command },
    );
    const displayedDialogCount = () => browser.execute(() =>
      [...document.querySelectorAll('[role="dialog"]')]
        .filter((node) => node.getClientRects().length > 0).length,
    );

    current = { id: "UX-B74", task: "Edit outline delete counts and character chapter filters", visited: [] };
    for (const width of [1440, 390]) {
      current.width = width;
      await browser.setWindowSize(width, height);

      let page = await project("大纲");
      await clickText("分部大纲", page);
      const openAdvanced = await textButton("高级工具", page);
      await openAdvanced.waitForEnabled({ timeout });
      await press(openAdvanced);
      let advanced = await dialog("大纲高级工具");
      await press(await advanced.$('details[aria-label="末尾维护与删除"] > summary'));
      for (const [label, command, confirmationTitle, confirmationText] of [
        ["删除最近分部", "delete_latest_parts", "删除分部大纲", "删除最近 2 个分部"],
        ["删除最近章纲", "delete_latest_chapter_outlines", "删除章纲", "删除最近 2 个章纲"],
      ]) {
        const section = await advanced.$('.//section[.//h3[normalize-space(.)=' + literal(label) + ']]');
        const count = await section.$('input[type="number"]');
        await clearByKeyboard(count);
        assert.equal(await count.getValue(), "", label + " must preserve the empty editing state");
        assert.deepEqual(await snapshot("UX-B74-delete-count", width, label + "-empty", advanced), []);
        const before = await requestCount(command);
        await press(await textButton("删除", section));
        const countError = await section.$('[role="alert"]');
        await countError.waitForDisplayed({ timeout });
        assert.match(await countError.getText(), /输入 1 ~/);
        assert.equal(await count.getAttribute("aria-invalid"), "true");
        await browser.waitUntil(async () => count.isFocused(), { timeout });
        assert.equal(await countError.execute((node) => {
          const rect = node.getBoundingClientRect();
          return rect.top >= 0 && rect.bottom <= innerHeight;
        }), true, "The full delete-count error must scroll into view");
        assert.deepEqual(await snapshot("UX-B74-delete-count", width, label + "-empty-error", advanced), []);
        assert.equal(await displayedDialogCount(), 1, "An empty count must not open destructive confirmation");
        assert.equal(await requestCount(command), before, "An empty count must not send a delete request");
        await count.setValue("2");
        assert.equal(await count.getValue(), "2");
        await press(await textButton("删除", section));
        const confirmation = await dialog(confirmationTitle);
        await waitText(confirmation, confirmationText);
        assert.deepEqual(await snapshot("UX-B74-delete-count", width, label + "-confirmation", confirmation), []);
        assert.equal(await requestCount(command), before, "Cancelling must not send a delete request");
        await clickText("取消", confirmation);
        await browser.waitUntil(async () => !await confirmation.isDisplayed(), { timeout });
      }
      await closeDialog();
      current.visited.push(width);

      let detail = await characterTab("管理", "属性");
      const maintain = await detail.$('.//details[.//summary[normalize-space(.)="手动维护属性"]]');
      await press(await maintain.$("summary"));
      const chapter = await maintain.$('input[type="number"]');
      const originalChapter = await chapter.getValue();
      await clearByKeyboard(chapter);
      assert.equal(await chapter.getValue(), "", "Event chapter edits must keep an empty draft");
      await chapter.setValue("7");
      assert.equal(await chapter.getValue(), "7", "Re-entry must replace the old chapter number");
      await chapter.setValue("2.5");
      const key = await maintain.$('.//label[.//span[normalize-space(.)="属性名称"]]/input');
      const cause = await maintain.$('.//label[.//span[normalize-space(.)="原因"]]/textarea');
      const evidence = await maintain.$('.//label[.//span[normalize-space(.)="证据"]]/textarea');
      await key.setValue("UX-B74 输入边界");
      await cause.setValue("输入的小数章节号必须被拒绝");
      await evidence.setValue("隔离 UI 夹具");
      const addCount = await requestCount("add_profile_attribute");
      await press(await textButton("新增属性", maintain));
      const chapterError = await maintain.$("#event-chapter-error");
      await chapterError.waitForDisplayed({ timeout });
      assert.match(await chapterError.getText(), /必须是 1 到 2,147,483,647 之间的整数/);
      assert.equal(await chapter.getAttribute("aria-invalid"), "true");
      assert.equal(await chapter.getValue(), "2.5", "Invalid text must stay visible for correction");
      assert.equal(await requestCount("add_profile_attribute"), addCount, "Invalid chapter numbers must not reach the write command");
      assert.deepEqual(await snapshot("UX-B74-event-chapter", width, "invalid-decimal", detail), []);
      await chapter.setValue("7");
      assert.equal(await chapter.getValue(), "7", "An integer re-entry must replace the previous chapter number");
      await chapter.setValue(originalChapter);
      await key.clearValue();
      await cause.clearValue();
      await evidence.clearValue();

      const filter = await detail.$('.//details[.//summary[contains(normalize-space(.),"筛选事件")]]');
      await press(await filter.$("summary"));
      const eventRange = await filter.$$('input[type="number"]');
      assert.equal(eventRange.length, 2);
      const applyEventFilter = await textButton("应用筛选", filter);
      await applyEventFilter.waitForEnabled({ timeout });
      let eventHistoryCount = await requestCount("attribute_history");
      await eventRange[0].setValue("2.5");
      await eventRange[1].setValue("3");
      await press(await textButton("应用筛选", filter));
      const eventFilterError = await detail.$("#event-filter-error");
      await eventFilterError.waitForDisplayed({ timeout });
      assert.match(await eventFilterError.getText(), /必须是 1 到 2,147,483,647 之间的整数/);
      assert.equal(await requestCount("attribute_history"), eventHistoryCount, "Fractional event filters must not be queried");
      await eventRange[0].setValue("3");
      await eventRange[1].setValue("1");
      await press(await textButton("应用筛选", filter));
      await browser.waitUntil(async () => (await eventFilterError.getText()).includes("章节范围无效"), { timeout });
      assert.equal(await requestCount("attribute_history"), eventHistoryCount, "Reversed event ranges must not be queried");
      await eventRange[0].setValue("1");
      await eventRange[1].setValue("3");
      await press(await textButton("应用筛选", filter));
      await waitForRequest("attribute_history", eventHistoryCount);
      assert.deepEqual(await latestRequest("attribute_history"), {
        profile_id: 1011, chapter_from: 1, chapter_to: 3,
        category: null, key: null, operation: null, limit: null,
      });
      assert.deepEqual(await snapshot("UX-B74-event-filter", width, "valid-range", detail), []);

      detail = await characterTab("管理", "快照");
      const snapshotRange = await detail.$$('input[type="number"]');
      assert.equal(snapshotRange.length, 2);
      const refreshSnapshots = await textButton("刷新", detail);
      await refreshSnapshots.waitForEnabled({ timeout });
      let snapshotCount = await requestCount("list_snapshots");
      const conflictCount = await requestCount("check_profile_sync_conflict");
      await snapshotRange[0].setValue("2.5");
      await snapshotRange[1].setValue("3");
      await press(refreshSnapshots);
      const snapshotError = await detail.$('[role="alert"]');
      await snapshotError.waitForDisplayed({ timeout });
      assert.match(await snapshotError.getText(), /必须是 1 到 2,147,483,647 之间的整数/);
      assert.equal(await requestCount("list_snapshots"), snapshotCount, "Fractional snapshot filters must not be queried");
      assert.equal(await requestCount("check_profile_sync_conflict"), conflictCount, "Invalid snapshot filters must not start adjacent reads");
      assert.deepEqual(await snapshot("UX-B74-snapshot-filter", width, "invalid-decimal", detail), []);
      await snapshotRange[0].setValue("3");
      await snapshotRange[1].setValue("2");
      await press(refreshSnapshots);
      await browser.waitUntil(async () => (await snapshotError.getText()).includes("章节范围无效"), { timeout });
      assert.equal(await requestCount("list_snapshots"), snapshotCount, "Reversed snapshot ranges must not be queried");
      assert.equal(await requestCount("check_profile_sync_conflict"), conflictCount);
      await snapshotRange[0].setValue("1");
      await snapshotRange[1].setValue("3");
      await press(refreshSnapshots);
      await waitForRequest("list_snapshots", snapshotCount);
      const snapshotRequest = await latestRequest("list_snapshots");
      assert.equal(snapshotRequest.profile_id, 1011);
      assert.equal(snapshotRequest.start_chapter, 1);
      assert.equal(snapshotRequest.end_chapter, 3);
      snapshotCount = await requestCount("list_snapshots");
      assert.deepEqual(await snapshot("UX-B74-snapshot-filter", width, "valid-range", detail), []);

      detail = await characterTab("管理", "行为");
      const behaviorChapter = await detail.$('input[type="number"]');
      const refreshBehaviors = await textButton("刷新", detail);
      await refreshBehaviors.waitForEnabled({ timeout });
      let behaviorCount = await requestCount("list_behaviors");
      await behaviorChapter.setValue("2.5");
      await press(refreshBehaviors);
      const behaviorError = await detail.$("#behavior-chapter-filter-error");
      await behaviorError.waitForDisplayed({ timeout });
      assert.match(await behaviorError.getText(), /必须是 1 到 2,147,483,647 之间的整数/);
      assert.equal(await behaviorChapter.getAttribute("aria-invalid"), "true");
      assert.equal(await requestCount("list_behaviors"), behaviorCount, "Fractional behavior filters must not be queried");
      assert.deepEqual(await snapshot("UX-B74-behavior-filter", width, "invalid-decimal", detail), []);
      await behaviorChapter.setValue("2");
      await press(refreshBehaviors);
      await waitForRequest("list_behaviors", behaviorCount);
      assert.deepEqual(await latestRequest("list_behaviors"), {
        profile_id: 1011, chapter_from: 2, chapter_to: 2, limit: 20,
      });
      behaviorCount = await requestCount("list_behaviors");
      current.visited.push(width);
    }
  });

  for (const [id, tab, expected] of [
    ["W01-body", "正文", null], ["W02-analysis", "分析", "林岚"],
    ["W03-images", "图像", "旧书店"], ["W04-director", "导演编排", "导演编排"],
    ["W05-script-desk", "拍摄剧本", "林岚"],
    ["W06-video-script", "拍摄剧本", "场景 1"],
  ]) {
    const selectedWidths = ["W02-analysis", "W03-images", "W04-director"].includes(id)
      ? [1440, 1024, 900, 390]
      : widths;
    it(id + (id === "W06-video-script" ? " opens the video project screenplay workspace" : " opens the independent chapter workspace"), async () => matrix({
      id,
      task: id === "W06-video-script" ? "从视频项目进入拍摄剧本工作面" : "在真实写作台切换到" + tab,
      open: async () => {
        const scope = id === "W06-video-script"
          ? await videoScreenplay()
          : tab === "导演编排"
            ? await videoDirector()
            : id === "W05-script-desk"
              ? await videoScreenplay()
            : await writing(tab);
        if (tab === "拍摄剧本") await openScriptScene(scope, 0);
        return scope;
      }, text: expected ? [expected] : [],
      check: id === "W04-director" ? async (_scope, width) => {
        await browser.pause(250);
        const threadError = await browser.execute(() => [...document.querySelectorAll(".a4n-toast")]
          .map((toast) => toast.textContent ?? "")
          .find((text) => text.includes("thread_id")) ?? null);
        assert.equal(threadError, null, "The director background bridge must not request an empty thread id");
        const stageSelector = await $('[role="combobox"][id="video-stages-3-select"]');
        const stageTabs = await _scope.$(".a4n-media-stage-tabs nav");
        assert.equal(await stageSelector.isDisplayed(), width < 1280, "Medium and narrow layouts must expose direct stage selection");
        assert.equal(await stageTabs.isDisplayed(), width >= 1280, "Wide layouts must retain the stage tab rail");
        const timeline = await _scope.$(".director-timeline-scroll");
        const timelineState = await timeline.execute((node) => ({
          clientWidth: node.clientWidth,
          scrollWidth: node.scrollWidth,
          scrollLeft: node.scrollLeft,
        }));
        const directorWidth = await _scope.$(".director-workspace").getSize().then(({ width }) => width);
        const compactTimeline = directorWidth <= 600;
        const timelineScrollable = timelineState.scrollWidth > timelineState.clientWidth;
        const scrollHint = await _scope.$(".director-timeline-scroll-hint");
        await browser.waitUntil(async () => (await scrollHint.isDisplayed()) === timelineScrollable, {
          timeout,
          timeoutMsg: "The horizontal-scroll cue must match the timeline's actual overflow",
        });
        if (compactTimeline) assert.ok(timelineScrollable, "The full timeline must remain horizontally scrollable");
        if (timelineScrollable) {
          const lastSegment = await _scope.$(".director-segment:last-child");
          const reachedLastSegment = await wheelTo(lastSegment, "end");
          assert.equal(reachedLastSegment.reached, true, "The final timeline segment must be user-scrollable");
          const end = await timeline.execute((node) => {
            const maxScrollLeft = node.scrollWidth - node.clientWidth;
            return { maxScrollLeft, scrollLeft: node.scrollLeft };
          });
          assert.ok(end.maxScrollLeft > 0, "The timeline must have later time ranges");
          assert.ok(end.scrollLeft >= end.maxScrollLeft - 1, "The timeline must reach its final time range");
          const endFindings = await snapshot("W04-director", width, "timeline-end", _scope, { timelineEnd: end, reachedLastSegment });
          assert.deepEqual(endFindings, [], "The final timeline viewport must remain visible and error-free");
          await timeline.execute((node, scrollLeft) => { node.scrollLeft = scrollLeft; }, timelineState.scrollLeft);
        }
        return { stageNavigation: width < 1280 ? "compact selector" : "tab rail", directorWidth, timeline: timelineState, scrollCueVisible: timelineScrollable };
      } : tab === "正文" ? async () => {
        const editor = await $('textarea[aria-label="章节正文"]');
        assert.equal(await editor.getValue(), seed.commands.list_chapters[0].content);
        const readEnd = () => editor.execute((element) => {
          const style = getComputedStyle(element);
          return {
            caret: element.selectionStart, selectionEnd: element.selectionEnd, length: element.value.length,
            focused: element === document.activeElement,
            scrollHeight: element.scrollHeight, clientHeight: element.clientHeight, scrollTop: element.scrollTop,
            remaining: element.scrollHeight - element.clientHeight - element.scrollTop,
            paddingTop: parseFloat(style.paddingTop), paddingBottom: parseFloat(style.paddingBottom),
            paddingLeft: parseFloat(style.paddingLeft), paddingRight: parseFloat(style.paddingRight),
            borderTop: parseFloat(style.borderTopWidth), borderBottom: parseFloat(style.borderBottomWidth),
            lineHeight: parseFloat(style.lineHeight), fontSize: parseFloat(style.fontSize),
            offsetHeight: element.offsetHeight, rectHeight: element.getBoundingClientRect().height,
          };
        });
        await press(editor);
        const samples = [{ phase: "before-shortcut", ...await readEnd() }];
        await browser.keys(["Control", "End"]);
        const started = Date.now();
        let end;
        try {
          await browser.waitUntil(async () => {
            end = await readEnd();
            samples.push({ phase: "after-shortcut", elapsedMs: Date.now() - started, ...end });
            current.evidence = { longChapterEnd: end, longChapterEndSamples: samples };
            // Ctrl+End exposes the final caret line; trailing padding and line-box space need not scroll to the pixel bottom.
            return end.focused && end.caret === end.length && end.remaining <= end.paddingBottom + end.lineHeight;
          }, { timeout, timeoutMsg: "The final text line must be reachable with Ctrl+End" });
        } catch (error) {
          throw new Error("The final text line must be reachable with Ctrl+End; last metrics: " + JSON.stringify(end), { cause: error });
        }
        assert.ok(end.focused, "Ctrl+End must leave keyboard focus in the chapter editor");
        assert.equal(end.caret, end.length);
        return { longChapterEnd: end, longChapterEndSamples: samples };
      } : undefined,
      last: ["拍摄剧本", "分析", "图像"].includes(tab) ? async (scope) => {
        if (tab === "拍摄剧本") {
          await press(await scope.$('details[class~="group/scene"] > summary'));
          return openScriptScene(scope, -1);
        }
        if (tab === "分析") {
          const tension = seed.commands.list_chapters[0].analysis_data.foreshadowing.tensions.at(-1);
          return scope.$('.//li[normalize-space(.)=' + literal(tension) + ']');
        }
        return (await scope.$$("li img")).at(-1);
      } : undefined,
    }, selectedWidths));
  }

  it("W01 keeps chapter editing keyboard-operable at a 702 x 393 viewport", async function () {
    this.timeout(60000);
    const id = "W01-keyboard-702css";
    current = { id, task: "键盘进入正文、200% 等效窄视口选章、编辑并保存", visited: [], plannedWidths: [702] };
    await browser.setWindowSize(1440, height);
    await home();

    const enterDesk = await (await rootRow()).$('button[aria-label$=" · 进入写作桌"]');
    await browser.execute(() => {
      if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    });
    let entryTabs = 0;
    while (!await enterDesk.isFocused() && entryTabs < 80) {
      await browser.keys("Tab");
      entryTabs += 1;
    }
    assert.equal(await enterDesk.isFocused(), true, "The writing desk entry must be reachable by Tab");
    await browser.keys("Enter");
    await $('textarea[aria-label="章节正文"]').waitForDisplayed({ timeout });

    const initialViewport = await browser.execute(() => ({
      devicePixelRatio, width: innerWidth, height: innerHeight, outerHeight,
    }));
    if (process.env.A4N_UI_ACTUAL_ZOOM_ONLY === "1") {
      const runXdotool = (args) => {
        const result = spawnSync("xdotool", args, { encoding: "utf8" });
        if (result.error) throw result.error;
        if (result.status !== 0) throw new Error(result.stderr || "xdotool command failed");
        return result.stdout ?? "";
      };
      const windowIds = runXdotool(["search", "--onlyvisible", "--class", "chromium"])
        .trim()
        .split(/\s+/)
        .filter(Boolean);
      assert.ok(windowIds.length > 0, "The visible Chromium window must be available for native browser zoom");
      const windowId = windowIds.at(-1);
      runXdotool(["windowactivate", "--sync", windowId]);
      assert.equal(runXdotool(["getwindowfocus"]).trim(), windowId, "The browser zoom shortcut must reach the active Chromium window");
      try {
        const actualZoomSteps = [];
        let actualBrowserZoom = null;
        for (const shortcut of ["ctrl+plus", "ctrl+shift+equal"]) {
          for (let step = 0; step < 6; step += 1) {
            runXdotool(["key", "--clearmodifiers", shortcut]);
            await browser.pause(150);
            actualBrowserZoom = await browser.execute(() => ({
              devicePixelRatio,
              innerWidth,
              innerHeight,
              visualScale: visualViewport?.scale ?? null,
            }));
            actualZoomSteps.push({ shortcut, step: step + 1, ...actualBrowserZoom });
            if (actualBrowserZoom.devicePixelRatio >= initialViewport.devicePixelRatio * 1.9
              && actualBrowserZoom.innerWidth <= initialViewport.width / 1.9) break;
          }
          if (actualBrowserZoom.devicePixelRatio >= initialViewport.devicePixelRatio * 1.9
            && actualBrowserZoom.innerWidth <= initialViewport.width / 1.9) break;
          runXdotool(["key", "--clearmodifiers", "ctrl+0"]);
          await browser.pause(150);
        }
        const layout = await browser.execute(() => {
          const box = (element) => {
            const bounds = element?.getBoundingClientRect();
            return bounds ? { top: bounds.top, right: bounds.right, width: bounds.width } : null;
          };
          const header = document.querySelector(".a4n-app-page-header--desk");
          const title = header?.querySelector("h1");
          const actions = header?.querySelector(".a4n-app-page-header__actions");
          return {
            innerWidth,
            innerHeight,
            overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
            editor: box(document.querySelector('textarea[aria-label="章节正文"]')),
            title: box(title),
            actions: box(actions),
          };
        });
        current.width = actualBrowserZoom?.innerWidth ?? null;
        current.evidence = { initialViewport, actualBrowserZoom, actualZoomSteps, layout };
        await browser.saveScreenshot(`${output}${phase}-W01-200-percent-browser-zoom.png`);
        assert.ok(actualBrowserZoom.devicePixelRatio >= initialViewport.devicePixelRatio * 1.9, "Native Chromium zoom must reach 200%");
        assert.ok(actualBrowserZoom.innerWidth <= initialViewport.width / 1.9, "Native 200% zoom must halve the CSS viewport width");
        assert.equal(layout.overflow, 0);
        assert.ok(layout.editor?.width > 0 && layout.editor.top < layout.innerHeight);
        assert.ok(layout.title?.right <= layout.innerWidth + 1);
        assert.ok(layout.actions?.right <= layout.innerWidth + 1);
        current.visited.push(current.width);
        return;
      } finally {
        runXdotool(["key", "--clearmodifiers", "ctrl+0"]);
        await browser.waitUntil(async () => {
          const state = await browser.execute(() => ({ devicePixelRatio, width: innerWidth }));
          return Math.abs(state.devicePixelRatio - initialViewport.devicePixelRatio) < 0.02
            && Math.abs(state.width - initialViewport.width) <= 8;
        }, { timeout, timeoutMsg: "Native browser zoom must reset before closing the audit session" });
      }
    }
    const browserFrameHeight = initialViewport.outerHeight - initialViewport.height;
    await browser.setWindowSize(702, 393 + browserFrameHeight);
    await browser.waitUntil(async () => (await browser.execute(() => innerWidth)) <= 710, { timeout });
    await browser.waitUntil(async () => Math.abs(await browser.execute(() => innerHeight) - 393) <= 5, { timeout });
    await browser.waitUntil(async () => await browser.execute(() => Boolean(document.querySelector('[role="combobox"]'))), {
      timeout,
      timeoutMsg: "The mobile chapter selector must appear after the viewport reflows",
    });

    const narrowViewport = await browser.execute(() => {
      const rect = (element) => {
        const box = element.getBoundingClientRect();
        return { left: box.left, top: box.top, right: box.right, bottom: box.bottom, width: box.width, height: box.height };
      };
      const header = document.querySelector(".a4n-app-page-header--desk");
      const title = header?.querySelector("h1");
      const projectTitle = title?.firstElementChild;
      const actions = header?.querySelector(".a4n-app-page-header__actions");
      const chapterToolbar = document.querySelector(".a4n-writing-chapter-toolbar");
      const chapterControls = chapterToolbar ? [...chapterToolbar.querySelectorAll("button")].map(rect) : [];
      const spacing = (element) => {
        if (!element) return null;
        const style = getComputedStyle(element);
        return {
          minHeight: style.minHeight,
          paddingBlockStart: style.paddingBlockStart,
          paddingBlockEnd: style.paddingBlockEnd,
          marginBlockStart: style.marginBlockStart,
          marginBlockEnd: style.marginBlockEnd,
        };
      };
      return {
        devicePixelRatio,
        innerWidth,
        innerHeight,
        outerWidth,
        scrollWidth: document.documentElement.scrollWidth,
        clientWidth: document.documentElement.clientWidth,
        editor: rect(document.querySelector('textarea[aria-label="章节正文"]')),
        title: title ? rect(title) : null,
        titleTextOverflow: projectTitle ? getComputedStyle(projectTitle).textOverflow : null,
        actions: actions ? rect(actions) : null,
        chapterToolbar: chapterToolbar ? rect(chapterToolbar) : null,
        chapterToolbarOverflow: chapterToolbar ? chapterToolbar.scrollWidth - chapterToolbar.clientWidth : null,
        chapterControls,
        chapterPickerVisible: [...document.querySelectorAll('[role="combobox"], select')]
          .some((element) => element.getBoundingClientRect().width > 0),
        spacing: {
          header: spacing(header),
          chapterToolbar: spacing(chapterToolbar),
          workspace: spacing(document.querySelector(".a4n-writing-workspace")),
          heading: spacing(document.querySelector(".a4n-writing-workspace-heading")),
          tabPanel: spacing(document.querySelector(".a4n-writing-tab-panel")),
          editor: spacing(document.querySelector('textarea[aria-label="章节正文"]')),
        },
      };
    });
    current.evidence = { initialViewport, narrowViewport };
    current.width = 702;
    assert.ok(initialViewport.width >= 1400, "The equivalent viewport must start from a 1440px layout");
    assert.ok(Math.abs(narrowViewport.innerWidth - 702) <= 8, "The writing desk must be measured at a 702px CSS width");
    assert.ok(Math.abs(narrowViewport.innerHeight - 393) <= 5, "The writing desk must be measured at a 393px CSS height");
    assert.ok(narrowViewport.innerWidth <= initialViewport.width / 1.9, "702 CSS px approximates a 1440px viewport at 200% zoom");
    assert.equal(narrowViewport.scrollWidth - narrowViewport.clientWidth, 0, "The writing desk must not overflow at the narrow viewport");
    assert.ok(narrowViewport.editor.width > 0, "The chapter editor must remain visible at the narrow viewport");
    assert.ok(narrowViewport.editor.top < narrowViewport.innerHeight, "The chapter editor must begin within the first viewport");
    assert.ok(narrowViewport.editor.height >= narrowViewport.innerHeight * 0.45, "Short viewports must reserve enough height for the chapter editor");
    assert.ok(narrowViewport.title?.width > 0 && narrowViewport.title.right <= narrowViewport.clientWidth + 1, "The project title must stay inside the page header");
    assert.equal(narrowViewport.titleTextOverflow, "ellipsis", "The project title must stay in a bounded single-line container");
    assert.ok(narrowViewport.actions?.width > 0 && narrowViewport.actions.right <= narrowViewport.clientWidth + 1, "Page actions must remain visible inside the viewport");
    assert.ok(narrowViewport.chapterToolbar?.width > 0 && narrowViewport.chapterToolbarOverflow === 0, "The chapter toolbar must not clip its controls");
    assert.ok(narrowViewport.chapterControls.every((control) => control.left >= 0 && control.right <= narrowViewport.clientWidth + 1), "Every chapter control must remain inside the viewport");
    assert.ok(narrowViewport.chapterControls.every((control) => control.height >= 44), "Compact chapter controls must preserve 44px touch targets");
    assert.equal(narrowViewport.chapterPickerVisible, true, "The chapter selector must remain available at the narrow viewport");

    const chapterPicker = await $('[role="combobox"], select');
    const tabTo = async (element, reverse = false) => {
      for (let count = 0; count < 80; count += 1) {
        if (await element.isFocused()) return count;
        await browser.keys(reverse ? ["Shift", "Tab"] : "Tab");
      }
      assert.fail("Keyboard focus did not reach " + await element.getAttribute("aria-label"));
    };
    const chapterPickerTabs = await tabTo(chapterPicker);
    await browser.keys("Enter");
    await $('[role="listbox"]').waitForDisplayed({ timeout });
    await browser.keys("Home");
    await browser.keys("ArrowDown");
    await browser.keys("Enter");
    await assertFocus(chapterPicker);

    const editor = await $('textarea[aria-label="章节正文"]');
    await browser.waitUntil(async () => (await editor.getValue()) === seed.commands.list_chapters[1].content, { timeout });
    const file = phase + "-" + id + "-702-css-viewport.png";
    await browser.saveScreenshot(output + file);
    const screenshot = await readFile(output + file);

    const editorTabs = await tabTo(editor);
    await browser.keys(["Control", "End"]);
    await browser.keys("x");
    const saveState = await $('.a4n-writing-save-state');
    await browser.waitUntil(async () => await saveState.getAttribute("data-state") === "dirty", { timeout });

    const saveButton = await $('button[aria-label="保存修改"]');
    await saveButton.waitForDisplayed({ timeout });
    const saveTabs = await tabTo(saveButton, true);
    const contentToSave = await editor.getValue();
    const saveFailureMessage = "正文保存暂时不可用，请重试。";
    await browser.execute(() => window.__uiAudit.deferred.push("update_chapter_content"));
    await browser.keys("Enter");
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "update_chapter_content")), { timeout });
    const failedRequest = await browser.execute(() => window.__uiAudit.pending.find((item) => item.command === "update_chapter_content").body.input);
    assert.equal(failedRequest.content, contentToSave, "The failed save must contain the current editor draft");
    const savingState = await browser.execute(() => ({
      state: document.querySelector(".a4n-writing-save-state")?.getAttribute("data-state"),
      disabled: document.querySelector('button[aria-label="保存修改"]')?.disabled,
    }));
    assert.deepEqual(savingState, { state: "saving", disabled: true }, "Saving must disable duplicate submissions and expose progress");
    await browser.execute((errorMessage) => {
      const audit = window.__uiAudit;
      const index = audit.pending.findIndex((item) => item.command === "update_chapter_content");
      const [request] = audit.pending.splice(index, 1);
      audit.deferred = audit.deferred.filter((command) => command !== "update_chapter_content");
      request.resolve(Response.json({ code: "unavailable", message: errorMessage, retryable: true, trace_id: null, details: {} }, { status: 503 }));
    }, saveFailureMessage);
    await browser.waitUntil(async () => await saveState.getAttribute("data-state") === "dirty", { timeout });
    const saveErrorToast = await $(".a4n-toast--error");
    await saveErrorToast.waitForDisplayed({ timeout });
    await waitText(saveErrorToast, saveFailureMessage);
    assert.equal(await editor.getValue(), contentToSave, "A failed save must preserve the editor draft");
    const focusAfterFailure = await browser.execute(() => ({
      tag: document.activeElement?.tagName ?? null,
      label: document.activeElement?.getAttribute("aria-label") ?? null,
    }));
    assert.deepEqual(focusAfterFailure, { tag: "TEXTAREA", label: "章节正文" }, "Keyboard save failure must return focus to the editor");
    const saveErrorFile = phase + "-" + id + "-save-error.png";
    await browser.saveScreenshot(output + saveErrorFile);
    await press(await saveErrorToast.$("button[aria-label]"));
    await browser.waitUntil(async () => !await saveErrorToast.isExisting(), { timeout });

    await browser.execute((content) => {
      const chapter = window.__uiAudit.fixtures.byChapter[2]?.get_chapter;
      if (!chapter) throw new Error("Missing chapter 2 save fixture");
      window.__uiAudit.fixtures.update_chapter_content = { ...chapter, content };
    }, contentToSave);
    const retryButton = await $('button[aria-label="保存修改"]');
    const retrySaveTabs = await tabTo(retryButton, true);
    await browser.keys("Enter");
    await browser.waitUntil(async () => await saveState.getAttribute("data-state") === "saved", { timeout });
    assert.deepEqual(await browser.execute(() => window.__uiAudit.unexpected), [], "Saving must use a declared fixture response");
    assert.equal(await $(".a4n-toast").isExisting(), false, "The saved state already confirms successful content saves");
    const focusAfterSave = await browser.execute(() => ({
      tag: document.activeElement?.tagName ?? null,
      label: document.activeElement?.getAttribute("aria-label") ?? null,
      text: document.activeElement?.textContent?.trim().slice(0, 80) ?? null,
    }));
    assert.equal(focusAfterSave.tag, "TEXTAREA", "Keyboard save must return focus to the chapter editor");
    assert.equal(focusAfterSave.label, "章节正文");

    await browser.setWindowSize(1024, height);
    await browser.waitUntil(async () => (await browser.execute(() => innerWidth)) >= 1000, { timeout });
    const mediumViewport = await browser.execute(() => {
      const rect = (element) => {
        const box = element.getBoundingClientRect();
        return { left: box.left, right: box.right, width: box.width };
      };
      const header = document.querySelector(".a4n-app-page-header--desk");
      return {
        innerWidth,
        innerHeight,
        scrollWidth: document.documentElement.scrollWidth,
        clientWidth: document.documentElement.clientWidth,
        title: header?.querySelector("h1") ? rect(header.querySelector("h1")) : null,
        actions: header?.querySelector(".a4n-app-page-header__actions")
          ? rect(header.querySelector(".a4n-app-page-header__actions"))
          : null,
        editor: document.querySelector('textarea[aria-label="章节正文"]')
          ? rect(document.querySelector('textarea[aria-label="章节正文"]'))
          : null,
      };
    });
    assert.ok(Math.abs(mediumViewport.innerWidth - 1024) <= 8, "The writing desk must be measured at a 1024px CSS width");
    assert.equal(mediumViewport.scrollWidth - mediumViewport.clientWidth, 0, "The writing desk must not overflow at 1024px");
    assert.ok(mediumViewport.title?.width > 0 && mediumViewport.title.right <= mediumViewport.clientWidth + 1, "The project title must fit the 1024px header");
    assert.ok(mediumViewport.actions?.width > 0 && mediumViewport.actions.right <= mediumViewport.clientWidth + 1, "Page actions must fit the 1024px header");
    assert.ok(mediumViewport.editor?.width > 0 && mediumViewport.editor.right <= mediumViewport.clientWidth + 1, "The chapter editor must fit the 1024px workspace");
    const mediumFile = phase + "-" + id + "-1024-css-viewport.png";
    await browser.saveScreenshot(output + mediumFile);

    const leaveDraft = contentToSave + "临时未保存离开保护测试";
    await editor.setValue(leaveDraft);
    await browser.waitUntil(async () => await saveState.getAttribute("data-state") === "dirty", { timeout });
    const beforeLeaveUrl = await browser.getUrl();
    const bodySaveRequestsBeforeLeave = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "update_chapter_content").length);
    assert.equal(bodySaveRequestsBeforeLeave, 2, "The retry should follow one failed chapter save");
    await press(await ariaButton("返回上一页"));
    const leaveGuard = await dialog("未保存修改");
    await waitText(leaveGuard, "返回前已自动保存本地草稿");
    const leaveGuardFile = phase + "-" + id + "-leave-guard.png";
    await browser.saveScreenshot(output + leaveGuardFile);
    await clickText("留在当前", leaveGuard);
    await browser.waitUntil(async () => await browser.getUrl() === beforeLeaveUrl, { timeout });
    assert.equal(await editor.getValue(), leaveDraft, "Canceling navigation must preserve the unsaved chapter draft");
    assert.equal(await saveState.getAttribute("data-state"), "dirty");
    await press(await ariaButton("返回上一页"));
    const leaveAgain = await dialog("未保存修改");
    await waitText(leaveAgain, "可随时恢复");
    await clickText("继续", leaveAgain);
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/", { timeout });
    await $(".a4n-home-content").waitForDisplayed({ timeout });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "update_chapter_content").length), bodySaveRequestsBeforeLeave, "Discarding on navigation must not save the draft");
    await browser.url("/write/1?chapter=2");
    const restorePrompt = await dialog("恢复草稿");
    await waitText(restorePrompt, "是否恢复到编辑器");
    const draftRestoreFile = phase + "-" + id + "-draft-restore.png";
    await browser.saveScreenshot(output + draftRestoreFile);
    await clickText("恢复草稿", restorePrompt);
    const restoredEditor = await $('textarea[aria-label="章节正文"]');
    await browser.waitUntil(async () => await restoredEditor.getValue() === leaveDraft, { timeout });
    const routeLeave = { stayPreservedDraft: true, continueSavedLocalDraft: true, restoredDraft: true, bodySaveRequests: bodySaveRequestsBeforeLeave };

    report.push({
      id,
      task: current.task,
      stage: "1440px-layout-equivalent-702-css-px-plus-1024px-overflow-check",
      file,
      mediumFile,
      saveErrorFile,
      leaveGuardFile,
      draftRestoreFile,
      browserZoom: false,
      initialViewport,
      narrowViewport,
      mediumViewport,
      keyboard: { entryTabs, chapterPickerTabs, editorTabs, saveTabs, retrySaveTabs, focusAfterFailure, focusAfterSave },
      saveFailure: { status: 503, dirtyDraftPreserved: true, retryEnabled: true },
      routeLeave,
      findings: [],
    });
    current.evidence = {
      initialViewport,
      narrowViewport,
      mediumViewport,
      keyboard: { entryTabs, chapterPickerTabs, editorTabs, saveTabs, retrySaveTabs, focusAfterFailure, focusAfterSave },
      saveFailure: { status: 503, dirtyDraftPreserved: true, retryEnabled: true },
      routeLeave,
      screenshotPixels: { width: screenshot.readUInt32BE(16), height: screenshot.readUInt32BE(20) },
    };
    await saveReport();

    await browser.setWindowSize(1440, height);
    await browser.waitUntil(async () => {
      const width = await browser.execute(() => innerWidth);
      return width >= initialViewport.width - 20;
    }, { timeout, timeoutMsg: "The original viewport width must be restored" });
    current.visited.push(702);
  });

  const inner = [
    { id: "N01-fact-feedback", task: "区分已更新、待采用和已撤回的事实变化，核对正文证据", open: async () => {
      const page = await project();
      const scope = await page.$('.//section[./header/h3[normalize-space()="正文事实反馈"]]');
      await waitText(scope, "已归纳 2 章 · 待归纳 1/3 章");
      await press(await scope.$("summary"));
      return scope;
    }, text: ["已更新", "待采用", "来源已撤回"], check: async (scope) => {
      const changes = seed.commands.get_project_fact_feedback.changes;
      assert.equal((await scope.$$("ol > li")).length, changes.length);
      assert.equal(await scope.$('input[type="number"]').getValue(), "3");
      const quotes = await scope.$$("blockquote");
      for (const [index, change] of changes.entries()) assert.ok((await quotes[index].getText()).includes(change.evidence[0].quote));
      return { changes: changes.length, states: changes.map((change) => change.status), evidenceMatchesChapter: true };
    }, last: async (scope) => (await scope.$$("blockquote")).at(-1) },
    { id: "N01-synopsis", task: "阅读完整梗概而非卡片截断内容", open: async () => {
      const scope = await project(); await clickText("查看完整", scope); return dialog();
    }, text: ["第三部分让缺齿暗记"], last: async (scope) => scope.$(".whitespace-pre-wrap") },
    { id: "N01-blueprint-edit", task: "检查已有蓝图编辑字段和最终操作", open: async () => {
      const scope = await project();
      await press(await scope.$('button[aria-label="更多操作"]'));
      await menuItem("编辑蓝图"); return dialog();
    }, text: ["编辑正式蓝图"], check: async (scope) => {
      const values = await Promise.all([...await scope.$$("textarea")].map((field) => field.getValue()));
      assert.ok(values.some((value) => value.includes("林岚离开故乡七年")), "Editor must contain the saved synopsis");
      return { populatedTextareas: values.filter(Boolean).length };
    }, last: async (scope) => (await scope.$$("button")).at(-1) },
    { id: "N01-versions", task: "查看真实版本记录并找到比较入口", open: async () => {
      const scope = await project(); await press(await containing("蓝图版本", scope)); return dialog();
    }, text: ["第一版蓝图", "第三站台"], check: async (scope) => {
      const refresh = await textButton("刷新版本", scope);
      assert.ok(await refresh.isDisplayed());
      const beforeRefresh = await browser.execute(() =>
        window.__uiAudit.commands.filter((command) => command === "list_blueprint_versions").length,
      );
      await press(refresh);
      await browser.waitUntil(async () => await browser.execute(() =>
        window.__uiAudit.commands.filter((command) => command === "list_blueprint_versions").length,
      ) > beforeRefresh, { timeout });
      const descriptions = seed.commands.list_blueprint_versions.map((version) => version.change_description);
      const clipped = await scope.execute((node, values) => [...node.querySelectorAll("li span")].filter((span) => values.includes(span.textContent) && span.scrollWidth > span.clientWidth + 1).map((span) => span.textContent), descriptions);
      assert.deepEqual(clipped, [], "The version's change description must be readable before selecting or restoring it");
      return { completeVersionDescriptions: descriptions.length };
    }, last: async (scope) => (await scope.$$("li")).at(-1) },
    { id: "N01-version-restore", task: "核对具体蓝图版本的覆盖说明并取消恢复", open: async (width) => {
      const page = await project(); await press(await containing("蓝图版本", page));
      const versions = await dialog("蓝图版本");
      const showConfirmation = async () => {
        await press(await (await versions.$$("li")).at(-1).$('button[aria-label="更多操作"]'));
        await menuItem("恢复此版本");
        return dialog("恢复蓝图版本");
      };
      const confirmation = await showConfirmation();
      await waitText(confirmation, "当前蓝图会被覆盖");
      assert.deepEqual(await snapshot("N01-version-restore", width, "before-cancel", confirmation), []);
      await clickText("取消", confirmation);
      await browser.waitUntil(async () => !await confirmation.isDisplayed(), { timeout });
      assert.equal((await versions.$$("li")).length, seed.commands.list_blueprint_versions.length);
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("restore_blueprint_version")), false);
      return showConfirmation();
    }, text: ["版本 1", "历史"], last: async (scope) => textButton("取消", scope) },
    { id: "N01-version-compare", task: "选择第一版与第三版，阅读蓝图差异", open: async () => {
      const page = await project(); await press(await containing("蓝图版本", page));
      const scope = await dialog("蓝图版本");
      await browser.waitUntil(async () => (await scope.$$('button[aria-pressed]')).length === 3, { timeout });
      const versions = await scope.$$('button[aria-pressed]');
      await press(versions.at(-1)); await press(versions[0]);
      await clickText("对比已选版本", scope);
      await waitText(scope, seed.commands.diff_blueprint_versions.diff.full_synopsis.new);
      return scope;
    }, text: [seed.commands.diff_blueprint_versions.diff.full_synopsis.old], check: async () => {
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "diff_blueprint_versions")?.body.input);
      assert.deepEqual(request, { project_id: 1, version_a: "1", version_b: "3" });
      return { comparedVersions: [request.version_a, request.version_b] };
    }, last: async (scope) => (await scope.$$(".whitespace-pre-wrap")).at(-1) },
    { id: "N01-members", task: "核对成员角色与待接受邀请", open: () => openProjectAction("协作成员"), text: ["协作编辑", "待加入的正文协作者"], check: async (scope) => {
      const roles = await scope.execute((node) => [...node.querySelectorAll("li .a4n-themed-select__value")].map((value) => ({ text: value.textContent, width: value.clientWidth, contentWidth: value.scrollWidth })));
      assert.ok(roles.some((role) => role.text === "只读成员"));
      assert.ok(roles.every((role) => role.contentWidth <= role.width), "Member permissions must remain fully readable");
      return { roles };
    }, last: async (scope) => (await scope.$$("li")).at(-1) },
    { id: "N01-stage", task: "查看阶段历史和可选推进/回退动作", open: async () => {
      const scope = await openProjectAction("项目阶段");
      await press(await containing("状态变更历史", scope, "summary"));
      return scope;
    }, text: ["三部分结构", "开始前三章"] },
    { id: "N01-stage-current", task: "打开阶段窗口后同步外部推进的状态，移除草稿专属入口", open: async () => {
      await project("概览", undefined, async () => browser.execute(() => {
        const audit = window.__uiAudit;
        const draftProject = {
          ...audit.fixtures.get_project,
          status: "draft",
          progress: { label: "草稿", detail: null },
        };
        audit.fixtures.get_project = draftProject;
        audit.fixtures.get_project_status_machine = {
          ...audit.fixtures.get_project_status_machine,
          current_status: "draft",
          forward_targets: ["blueprint_ready"],
          rollback_targets: [],
        };
      }));
      await press(await $("button[title=\"更多项目操作\"]"));
      const actions = await dialog();
      await waitText(actions, "灵感");
      await browser.execute(() => {
        const audit = window.__uiAudit;
        audit.fixtures.get_project = {
          ...audit.fixtures.get_project,
          status: "completed",
          progress: { label: "已完成", detail: null },
        };
        audit.fixtures.get_project_status_machine = {
          ...audit.fixtures.get_project_status_machine,
          current_status: "completed",
          forward_targets: [],
          rollback_targets: ["writing", "chapter_outlines_ready", "part_outlines_ready", "blueprint_ready", "draft"],
        };
      });
      await press(await containing("项目阶段", actions));
      const scope = await dialog("项目阶段");
      await waitText(scope, "已完成");
      return scope;
    }, check: async (scope) => {
      await browser.keys(["ESC"]);
      await browser.waitUntil(async () => !await scope.isDisplayed(), { timeout });
      await press(await $("button[title=\"更多项目操作\"]"));
      const actions = await dialog();
      await browser.waitUntil(async () => !await containing("灵感", actions).isExisting(), { timeout });
      await browser.keys(["ESC"]);
      await browser.waitUntil(async () => !await actions.isDisplayed(), { timeout });
      return { dialogShowsLatestStage: true, draftOnlyActionRemoved: true };
    }, last: async () => $("button[title=\"更多项目操作\"]") },
    { id: "N01-stage-flow", task: "展开完整项目流程并阅读当前阶段至末端阶段", open: async () => {
      const scope = await openProjectAction("项目阶段");
      await press(await containing("查看完整流程", scope, "summary"));
      return scope;
    }, check: async (scope) => {
      const flow = await scope.$('[aria-label="项目阶段摘要选项"]');
      assert.equal((await flow.$$("li")).length, 6);
      assert.equal((await flow.$$('[aria-current="step"]')).length, 1);
      return { stages: 6, currentStage: await flow.$('[aria-current="step"]').getText() };
    }, last: async (scope) => (await scope.$$('[aria-label="项目阶段摘要选项"] li')).at(-1) },
    { id: "N01-rollback", task: "查看回滚的影响数量与章节范围，然后取消而不改动项目", open: async () => {
      const scope = await openProjectAction("项目阶段");
      await press(await containing("回滚到先前阶段", scope, "summary"));
      await press(await scope.$('button[role="combobox"]'));
      const targets = seed.commands.get_project_status_machine.rollback_targets;
      const options = await $$('[role="option"]');
      assert.equal(options.length, targets.length + 1);
      await press(options[targets.indexOf("blueprint_ready") + 1]);
      await waitText(scope, seed.commands.preview_rollback.impacts[0].title);
      return scope;
    }, check: async (scope, width) => {
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "preview_rollback")?.body);
      assert.deepEqual(request, { projectId: 1, expectedFrom: "writing", target: "blueprint_ready" });
      const pathPresentation = await browser.execute(() => {
        const path = document.querySelector(".a4n-rollback-path");
        return {
          statuses: path?.children.length ?? 0,
          arrows: [...(path?.querySelectorAll("svg.lucide-arrow-right") ?? [])]
            .map((icon) => icon.getAttribute("aria-hidden")),
          directions: [...(path?.querySelectorAll(".sr-only") ?? [])].map((item) => item.textContent.trim()),
        };
      });
      const transitionCount = seed.commands.preview_rollback.path.length - 1;
      assert.equal(pathPresentation.statuses, seed.commands.preview_rollback.path.length);
      assert.equal(pathPresentation.arrows.length, transitionCount);
      assert.deepEqual(pathPresentation.arrows, Array(transitionCount).fill("true"));
      assert.deepEqual(pathPresentation.directions, Array(transitionCount).fill("到"));
      for (const impact of seed.commands.preview_rollback.impacts) {
        await waitText(scope, impact.title);
        await waitText(scope, impact.note);
        await waitText(scope, `第 ${impact.chapter_start}-${impact.chapter_end} 章`);
      }
      assert.ok((await wheelTo(await textButton("确认回滚", scope))).reached);
      assert.deepEqual(await snapshot("N01-rollback", width, "impact-preview", scope, { request }), []);
      await clickText("取消", scope);
      assert.equal(await (await textButton("确认回滚", scope)).isExisting(), false);
      assert.equal(await scope.$('button[role="combobox"]').getText(), "选择要恢复的阶段");
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("rollback_project_status")), false);
      return { affectedGroups: seed.commands.preview_rollback.impacts.length, cancelledWithoutMutation: true };
    } },
    { id: "N01-rollback-stale", task: "阶段已变化后确认旧回滚预览，刷新状态机并清除过期确认", open: async () => {
      const scope = await openProjectAction("项目阶段");
      await press(await containing("回滚到先前阶段", scope, "summary"));
      await press(await scope.$('button[role="combobox"]'));
      const targets = seed.commands.get_project_status_machine.rollback_targets;
      const options = await $$('[role="option"]');
      await press(options[targets.indexOf("blueprint_ready") + 1]);
      await waitText(scope, seed.commands.preview_rollback.impacts[0].title);
      return scope;
    }, check: async (scope) => {
      await browser.execute(() => {
        const audit = window.__uiAudit;
        audit.fixtures.get_project = {
          ...audit.fixtures.get_project,
          status: "completed",
          progress: { label: "已完成", detail: null },
        };
        audit.fixtures.get_project_status_machine = {
          ...audit.fixtures.get_project_status_machine,
          current_status: "completed",
          forward_targets: [],
          rollback_targets: ["writing", "chapter_outlines_ready", "part_outlines_ready", "blueprint_ready", "draft"],
        };
        audit.failNext.push({ command: "rollback_project_status", status: 409, message: "项目状态已变化" });
      });
      await clickText("确认回滚", scope);
      await waitText($("body"), "项目状态已变化");
      await browser.waitUntil(async () => (await scope.$('button[role="combobox"]').getText()) === "选择要恢复的阶段", { timeout });
      const requests = await browser.execute(() => window.__uiAudit.requests.filter((item) => ["preview_rollback", "rollback_project_status"].includes(item.command)));
      assert.deepEqual(requests.at(-2).body, { projectId: 1, expectedFrom: "writing", target: "blueprint_ready" });
      assert.deepEqual(requests.at(-1).body, { projectId: 1, expectedFrom: "writing", target: "blueprint_ready" });
      assert.equal(await (await textButton("确认回滚", scope)).isExisting(), false);
      assert.equal(await scope.$(".a4n-context-bar strong").getText(), "已完成");
      return { staleMutationRejected: true, previewCleared: true, statusMachineReloaded: true };
    }, last: async (scope) => scope.$(".a4n-context-bar strong") },
    { id: "N01-export", task: "找到格式、元数据和结构化导出选项", open: () => openProjectAction("导出小说"), text: ["导出"], check: async (scope, width) => {
      const format = await scope.$('button[role="combobox"]');
      for (const [value, label, metadata] of [["txt", "TXT 纯文本", false], ["markdown", "Markdown", true], ["epub", "EPUB 电子书", true], ["pdf", "PDF", false], ["batch", "TXT + Markdown", true]]) {
        await press(format);
        await press(await $('//*[@role="option" and normalize-space(.)=' + literal(label) + ']'));
        const fields = await scope.$$('input[type="text"]');
        assert.equal(fields.length, metadata ? 2 : 0);
        if (metadata) {
          await fields[0].setValue(rootTitle);
          await fields[1].setValue("界面审查作者");
        }
        assert.deepEqual(await snapshot("N01-export", width, "format-" + value, scope, { metadataFields: fields.length }), []);
      }
      await press(await containing("结构数据", scope, "summary"));
      assert.equal((await scope.$$("details button")).length, 2);
      return { formats: 5, structuredExports: 2, downloaded: false };
    }, last: async (scope) => (await scope.$$("details button")).at(-1) },
    { id: "N01-lineage", task: "检查有来源与下游关系的内容图", open: () => openProjectAction("内容关联"), text: ["内容"], check: async (scope, width) => {
      await scope.$("canvas").waitForDisplayed({ timeout });
      const zoomControls = await scope.execute((dialog) => {
        const canvas = dialog.querySelector("canvas");
        const viewport = canvas?.parentElement?.parentElement;
        const buttons = [
          dialog.querySelector('button[aria-label="缩小依赖图"]'),
          dialog.querySelector('button[aria-label="放大依赖图"]'),
        ];
        if (!viewport || buttons.some((button) => !button)) return null;
        const view = viewport.getBoundingClientRect();
        const controls = buttons.map((button) => {
          const rect = button.getBoundingClientRect();
          return {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
            visible: rect.left >= view.left - 1 && rect.right <= view.right + 1
              && rect.top >= view.top - 1 && rect.bottom <= view.bottom + 1,
          };
        });
        return { controls, visible: controls.every((control) => control.visible) };
      });
      assert.ok(zoomControls?.visible, "Zoom controls must remain visible inside the graph viewport");
      const graphViewport = await scope.$("canvas").$("..").$("..");
      assert.equal(await graphViewport.getAttribute("role"), "region");
      assert.equal(await graphViewport.getAttribute("aria-label"), "画布已聚焦，可用方向键平移");
      await graphViewport.execute((node) => node.focus());
      const keyboardPan = await browser.execute(() => ({
        before: document.querySelector("canvas")?.closest(".overflow-auto")?.scrollLeft ?? 0,
        focused: document.activeElement?.getAttribute("aria-label"),
      }));
      await browser.keys("ArrowRight");
      await browser.pause(180);
      const keyboardPanAfter = await browser.execute(() => ({
        after: document.querySelector("canvas")?.closest(".overflow-auto")?.scrollLeft ?? 0,
        focused: document.activeElement?.getAttribute("aria-label"),
      }));
      assert.equal(keyboardPanAfter.focused, "画布已聚焦，可用方向键平移");
      assert.ok(keyboardPanAfter.after >= keyboardPan.before, "Focused graph canvas must respond to arrow-key panning");
      const panningState = await scope.execute((dialog) => {
        const hint = dialog.querySelector(".a4n-asset-graph-pan-hint");
        const viewport = dialog.querySelector("canvas")?.closest(".overflow-auto");
        return {
          hintVisible: Boolean(hint && getComputedStyle(hint).display !== "none"),
          hasHorizontalOverflow: Boolean(
            viewport && viewport.scrollWidth > viewport.clientWidth + 1,
          ),
        };
      });
      assert.equal(
        panningState.hintVisible,
        panningState.hasHorizontalOverflow,
        "Canvas panning guidance must match the graph viewport's actual overflow",
      );
      assert.deepEqual(await snapshot("N01-lineage", width, "graph", scope, { zoomControls, keyboardPan: { ...keyboardPan, ...keyboardPanAfter }, ...panningState }), []);
      if (width < 768) await clickText("资产详情", scope);
      const aside = await scope.$("aside");
      await aside.waitForDisplayed({ timeout });
      const node = seed.commands.get_project_asset_dependency_graph.nodes.find((item) => item.id === "blueprint:1");
      await press(await aside.$('button[title=' + JSON.stringify(node.label) + ']'));
      await waitText(aside, "来源记录");
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "get_project_asset_dependency_graph_node_detail")?.body);
      assert.equal(request.nodeId, node.id);
      const detail = await aside.$(":scope > section:last-child");
      assert.ok((await reach(detail)).reached);
      assert.deepEqual(await snapshot("N01-lineage", width, "asset-details", scope, { nodeId: node.id }), []);
      await press(await detail.$("summary"));
      const inputs = await detail.$('button[title="查看生成该资产的完整输入包"]');
      await press(inputs);
      const viewer = await dialog("生成输入");
      assert.deepEqual(await snapshot("N01-lineage", width, "generation-inputs", viewer, { inputs: seed.byAssetNode[node.id].records[0].generation_inputs.length }), []);
      await closeDialog();
      for (const summary of await detail.$$("details > summary")) {
        if (await summary.execute((node) => !node.parentElement.open)) await press(summary);
      }
      return { expectedNodes: seed.commands.get_project_asset_dependency_graph.nodes.length, selectedNodeId: node.id, inputViewerOpened: true };
    }, last: async (scope) => {
      const name = seed.commands.get_blueprint.world_setting.key_locations.at(-1).name;
      const lastLocation = await scope.$('.//aside//div[normalize-space(.)=' + literal(name) + ']');
      assert.ok(await lastLocation.isExisting(), "The world-setting input tree must render its last location");
      return lastLocation;
    } },
    ...["结构化", "JSON", "纯文本"].map((mode, index) => ({
      id: "N02-editor-" + index, task: "检查世界观" + mode + "编辑及末端字段",
      open: async () => { const scope = await project("世界观"); await clickText("编辑世界观", scope); await clickText(mode, scope); return scope; },
      text: ["保存世界观"],
      check: async (scope) => {
        const fields = await scope.$$("textarea");
        assert.ok(fields.length > 0);
        assert.ok((await fields[0].getValue()).length > 30, "World editor must have real data");
        const textareaHeights = await Promise.all([...fields].map(async (field) => (await field.getSize()).height));
        assert.ok(textareaHeights.every((value) => value >= (mode === "结构化" ? 120 : 280)), "World editing needs a usable multi-line area, not a two-line input");
        return { editingMode: mode, textareaCount: fields.length, textareaHeights };
      }, last: async (scope) => {
        const field = (await scope.$$("textarea")).at(-1);
        await press(field);
        await browser.keys(["Control", "End"]);
        const end = await field.execute((element) => ({
          caret: element.selectionEnd, length: element.value.length,
          remaining: element.scrollHeight - element.clientHeight - element.scrollTop,
          allowance: parseFloat(getComputedStyle(element).paddingBottom) + parseFloat(getComputedStyle(element).lineHeight),
        }));
        assert.equal(end.caret, end.length);
        assert.ok(end.remaining <= end.allowance, "The final world-setting line must be visible: " + JSON.stringify(end));
        await reach(field);
        const saveVisibility = await scope.execute((root) => {
          const button = [...root.querySelectorAll("button")].find((item) => item.textContent?.trim() === "保存世界观");
          const header = root.querySelector("section.a4n-flat-surface > header");
          if (!button || !header) return null;
          const action = button.getBoundingClientRect();
          const viewport = root.getBoundingClientRect();
          const stickyHeader = header.getBoundingClientRect();
          const background = getComputedStyle(header).backgroundColor.match(/[\d.]+/g) ?? [];
          return {
            visible: action.width > 0 && action.height > 0
              && action.top >= viewport.top - 1 && action.bottom <= viewport.bottom + 1,
            action: { top: action.top, bottom: action.bottom },
            viewport: { top: viewport.top, bottom: viewport.bottom },
            headerPosition: getComputedStyle(header).position,
            headerTop: stickyHeader.top,
            headerBackgroundAlpha: background.length > 3 ? Number(background[3]) : 1,
          };
        });
        assert.ok(saveVisibility?.visible, "The world-setting save action must stay visible at the final field: " + JSON.stringify(saveVisibility));
        if (await browser.execute(() => innerWidth < 1024)) {
          assert.equal(saveVisibility.headerPosition, "sticky", "The editor action header must remain anchored below 1024px");
          assert.equal(saveVisibility.headerBackgroundAlpha, 1, "The sticky editor header must not reveal scrolled field text");
        }
        return field;
      },
    })),
    { id: "N02-detail", task: "查看摘要之外的完整世界观字段", open: async () => {
      const scope = await project("世界观"); await clickText("查看完整设定", scope); return dialog();
    }, text: ["林岚尚不能确认镜中人身份"], captureInitialBeforeCheck: true, check: async (scope, width) => {
      for (const key of ["key_locations", "factions"]) {
        const item = seed.commands.get_blueprint.world_setting[key].at(-1);
        const label = await scope.$('.//*[normalize-space(.)=' + literal(item.name) + ']');
        const row = await label.$("..");
        assert.ok((await reach(row)).reached, "The last world entry must be scrollable: " + item.name);
        assert.deepEqual(await snapshot("N02-detail", width, key + "-last", scope, { finalEntry: item.name }), []);
      }
      const diagnostics = await scope.$("[data-modal-body] > details");
      await press(await diagnostics.$("summary"));
      const value = await diagnostics.$("pre").getText();
      assert.deepEqual(JSON.parse(value), seed.commands.get_blueprint.world_setting);
      return { completeWorldJson: true };
    }, last: async (scope) => scope.$("[data-modal-body] > details > pre") },
    { id: "N03-create", task: "发现创建角色的必填与可选输入", open: async () => {
      const scope = await project("角色"); await clickText("新建角色", scope); return dialog();
    }, text: ["角色"], check: async (scope) => {
      assert.ok((await scope.$$("input,textarea")).length >= 4, "Role form fields missing");
      return { inputs: (await scope.$$("input,textarea")).length };
    }, last: async (scope) => (await scope.$$("button")).at(-1) },
    { id: "N03-arc", task: "阅读角色弧光与阶段", open: () => characterTab("弧光"), text: ["从等待答案到主动核验", "主动承担下一步行动"], check: async (scope) => {
      const arc = seed.commands.get_character_arc_status;
      const stages = await scope.$$("ol li");
      assert.deepEqual(await Promise.all([...stages].map((stage) => stage.getText())), arc.stages);
      assert.equal(await scope.$$(".bg-book-primary\\/75").length, 1, "Current arc progress must have one visual bar");
      return { stageCount: stages.length, progress: arc.progress };
    }, last: async (scope) => (await scope.$$("ol li")).at(-1) },
    { id: "N03-voice", task: "阅读声音指纹和逐条示例", open: () => characterTab("声音指纹"), text: ["以具体问题替代情绪指控", "这封信，你为什么留到今天"], check: async (scope) => {
      const voice = seed.commands.get_character_voice_fingerprint;
      const lines = await scope.$$("ul > li");
      assert.deepEqual(await Promise.all([...lines].map((line) => line.getText())), [...voice.catchphrases, ...voice.examples]);
      return { separateExampleLines: lines.length };
    }, last: async (scope) => {
      const diagnostics = await containing("诊断信息", scope, "summary");
      if (!(await scope.$("details").getAttribute("open"))) await press(diagnostics);
      return scope.$("details pre");
    } },
    { id: "N03-consistency", task: "检查角色一致性并对照具体章节的预期与实际证据", open: async () => {
      const scope = await characterTab("一致性");
      await waitText(scope, "本次检查范围：全项目");
      await browser.execute(() => { window.__uiAudit.fixtures.get_workflow_progress_events = { events: [], stream_text: null }; });
      await clickText("开始检测", scope);
      const progress = await dialog("角色一致性检查");
      await waitText(progress, "任务已完成");
      await closeDialog();
      await waitText(scope, "第三章已抵达旧车站第三站台");
      return scope;
    }, captureInitialBeforeCheck: true, check: async (scope) => {
      const conflicts = seed.byProfile[1011].check_character_conflict;
      assert.equal((await scope.$$("li")).length, conflicts.length);
      for (const conflict of conflicts) for (const value of [conflict.expected, conflict.actual]) await waitText(scope, value);
      assert.ok((await scope.getText()).includes("本次范围：全项目"), "The result must state that this run covers the whole project");
      const rangeInputs = await scope.$$('input[type="number"]');
      assert.equal(rangeInputs.length, 2);
      await rangeInputs[0].setValue("1");
      await rangeInputs[1].setValue("3");
      await waitText(scope, "尚未检测一致性");
      assert.equal((await scope.$$("li")).length, 0, "Changing the range must clear stale findings");
      await clickText("开始检测", scope);
      const progress = await dialog("角色一致性检查");
      await waitText(progress, "任务已完成");
      await closeDialog();
      await waitText(scope, "第三章已抵达旧车站第三站台");
      const scopedText = await scope.getText();
      assert.ok(scopedText.includes("本次范围：第 1-3 章"), "Custom chapter ranges must be reflected beside their results");
      for (const conflict of conflicts) for (const value of [conflict.expected, conflict.actual]) await waitText(scope, value);
      return { profileId: 1011, conflicts: conflicts.length, checkedScopes: ["全项目", "第 1-3 章"], staleFindingsCleared: true };
    }, last: async (scope) => (await scope.$$("li")).at(-1) },
    { id: "N03-portraits", task: "查找已有立绘并浏览全部角色筛选", open: portraits, text: ["林岚", "周衡"], check: async (scope, width) => {
      const cards = await scope.$$('button[title="管理人物三视图拆解"]');
      assert.equal(cards.length, 4, "Need the four populated portraits");
      let expressionCard;
      for (const card of cards) {
        if ((await card.getText()).includes("疲惫而坦诚")) expressionCard = card;
      }
      assert.ok(expressionCard, "The distinct expression variant must be identifiable");
      const metadata = await expressionCard.$(".portrait-card-metadata").execute((root) =>
        [...root.children].map((item) => ({
          text: item.textContent?.trim(),
          clientWidth: item.clientWidth,
          scrollWidth: item.scrollWidth,
        })),
      );
      assert.ok(metadata.length >= 2 && metadata.every((item) => item.scrollWidth <= item.clientWidth + 1), "Portrait type and expression must wrap instead of being clipped");
      const clippedMetadata = await scope.execute((root) =>
        [...root.querySelectorAll('button[title="管理人物三视图拆解"] .mt-1.flex.flex-wrap > span')]
          .filter((label) => label.scrollWidth > label.clientWidth + 1)
          .map((label) => label.textContent?.trim()),
      );
      assert.deepEqual(clippedMetadata, [], "Portrait type and expression labels must remain readable");
      const misalignedRows = await scope.execute((root) => {
        const rows = new Map();
        for (const card of root.querySelectorAll('button[title="管理人物三视图拆解"]')) {
          const rect = card.getBoundingClientRect();
          const top = Math.round(rect.top);
          const row = rows.get(top) ?? [];
          row.push(Math.round(rect.bottom));
          rows.set(top, row);
        }
        return [...rows.values()].filter((bottoms) =>
          bottoms.length > 1 && Math.max(...bottoms) - Math.min(...bottoms) > 1,
        );
      });
      assert.deepEqual(misalignedRows, [], "Portrait cards in the same row must align at the bottom");
      const filters = await scope.$$(".a4n-filter-chips button");
      assert.equal(filters.length, seed.commands.list_profiles.length + 1);
      const lastFilter = filters.at(-1);
      const filterLayout = await scope.$(".a4n-filter-chips").execute((container) => {
        const bounds = container.getBoundingClientRect();
        const visible = [...container.querySelectorAll("button")].every((button) => {
          const rect = button.getBoundingClientRect();
          return rect.left >= bounds.left && rect.right <= bounds.right;
        });
        return { clientWidth: container.clientWidth, scrollWidth: container.scrollWidth, visible };
      });
      assert.ok(filterLayout.scrollWidth <= filterLayout.clientWidth + 2, "Character filters must not require horizontal scrolling");
      assert.equal(filterLayout.visible, true, "Every character filter must be visible without scrolling");
      await press(lastFilter);
      assert.equal(await lastFilter.getAttribute("aria-pressed"), "true");
      assert.deepEqual(await snapshot("N03-portraits", width, "last-filter", scope, { selectedProfile: await lastFilter.getText() }), []);
      await press(filters[0]);
      assert.equal((await scope.$$('button[title="管理人物三视图拆解"]')).length, 4);
      return { portraits: cards.length, reachableFilters: filters.length };
    }, last: async (scope) => (await scope.$$('button[title="管理人物三视图拆解"]')).at(-1) },
    { id: "N03-portrait-detail", task: "点击立绘图片中央进入素材管理", open: async () => {
      const scope = await portraits();
      await press(await scope.$('button[title="管理人物三视图拆解"] img'));
      return dialog("立绘详情");
    }, text: ["查看提示词"], check: async (scope, width) => {
      assert.equal(await $(".a4n-image-lightbox-panel").isDisplayed(), false, "Image center must open portrait details");
      const requestButton = await containing("查看重绘模型请求", scope);
      const redrawButton = await textButton("重绘立绘", scope);
      assert.equal(await requestButton.isDisplayed(), true, "The existing request preview must be directly discoverable");
      assert.equal(await requestButton.isEnabled(), true, "Request preview remains available when image size is ready");
      assert.equal(await scope.$('button[aria-haspopup="menu"]').isExisting(), false, "A single auxiliary action should not be hidden in an overflow menu");
      await browser.execute(() => {
        const generated = window.__uiAudit.fixtures.preview_generate_portrait_request;
        window.__uiAudit.fixtures.preview_regenerate_portrait_request = {
          ...structuredClone(generated),
          title: "立绘重绘请求",
          workflow: "portrait_regenerate",
          steps: generated.steps.map((step) => ({ ...step, title: "立绘重绘请求" })),
        };
      });
      await press(requestButton);
      const preview = await dialog("立绘重绘请求");
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "preview_regenerate_portrait_request")?.body.input);
      assert.ok(request?.portrait_id > 0);
      assert.ok(request?.width > 0 && request?.height > 0);
      assert.deepEqual(request?.reference_image_ids, []);
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("start_portrait_generation")), false, "Preview must not start generation");
      assert.deepEqual(await snapshot("N03-portrait-detail", width, "request-preview", preview, { previewCommand: "preview_regenerate_portrait_request", portraitId: request.portrait_id }), []);
      await closeDialog();
      await assertFocus(requestButton);
      return { imageCenterOpenedDetails: true, requestPreviewVisible: true, focusRestored: true, imageSize: { width: request.width, height: request.height } };
    }, last: async (scope) => textButton("重绘立绘", scope) },
    { id: "N03-portrait-age", task: "基于明确人物填写年龄段，保留原立绘并取消生成", open: async () => {
      const scope = await portraits();
      await press(await scope.$('button[title="管理人物三视图拆解"] img'));
      await press(await containing("生成年龄变体", await dialog("立绘详情")));
      const form = await dialog("生成年龄变体");
      const input = await form.$("input");
      assert.equal(await input.execute((node) => node.labels?.[0]?.textContent?.trim()), "年龄段");
      assert.equal(await (await textButton("生成变体", form)).isEnabled(), false);
      await input.setValue("35 岁");
      await (await textButton("生成变体", form)).waitForEnabled({ timeout });
      return form;
    }, text: ["林岚"], last: async (scope) => textButton("生成变体", scope), cleanup: async (scope) => {
      await browser.keys("Escape");
      const confirmation = await dialog("未保存修改");
      await clickText("取消", confirmation);
      assert.equal(await scope.$("input").getValue(), "35 岁", "Cancel must preserve the entered age");
      await browser.keys("Escape");
      const discard = await dialog("未保存修改");
      await clickText("放弃草稿", discard);
      await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"]')]
        .some((item) => item.textContent?.includes("生成年龄变体"))));
      return { closeConfirmationPreservedDraft: true, discardClosedDialog: true };
    } },
    { id: "N03-portrait-prompt", task: "读取当前人物的完整立绘提示词，保留渲染和请求预览入口", open: async () => {
      const scope = await portraits();
      const picture = await scope.$('button[title="管理人物三视图拆解"] img');
      const src = await picture.getAttribute("src");
      const expected = await browser.execute((source) => window.__uiAudit.fixtures.byProject[1].list_portraits_by_project.find((item) => item.image_path === source), src);
      assert.ok(expected?.prompt?.trim());
      await press(picture);
      await press(await containing("查看提示词", await dialog("立绘详情")));
      const form = await dialog("人物三视图拆解提示词");
      assert.equal(await form.$("pre").getText(), expected.prompt);
      return form;
    }, text: ["林岚", "当前提示词"], last: async (scope) => scope.$("pre"), cleanup: async () => {
      const portraitCard = await $('.a4n-page-container button[title="管理人物三视图拆解"]');
      await closeDialog();
      await assertFocus(portraitCard);
      return { closedWithEscape: true, focusReturnedToPortraitCard: true };
    } },
    { id: "N03-portrait-upload", task: "为明确人物选择本地图片，区分重新选图与独立预览", open: async () => {
      const scope = await portraits();
      await press(await scope.$('button[aria-label="更多操作"]'));
      await menuItem("上传立绘素材");
      const form = await dialog("上传角色立绘");
      assert.equal(await (await textButton("上传立绘", form)).isEnabled(), false);
      const file = await form.$('input[type="file"]');
      await file.setValue(fileURLToPath(new URL("../../resources/thelittleprince.png", import.meta.url)));
      const preview = await form.$("label img");
      await browser.waitUntil(async () => preview.execute((node) => node.complete && node.naturalWidth > 0), { timeout });
      await file.execute((node) => {
        window.__portraitFileClicks = 0;
        node.addEventListener("click", (event) => { window.__portraitFileClicks++; event.preventDefault(); });
      });
      await press(preview);
      assert.equal(await browser.execute(() => window.__portraitFileClicks), 1);
      assert.equal(await $(".a4n-image-lightbox-panel").isDisplayed(), false);
      const zoom = await ariaButton("图片预览", form);
      await press(zoom); await lightbox(); await closeDialog(); await assertFocus(zoom);
      assert.equal(await browser.execute(() => window.__portraitFileClicks), 1, "Independent preview must not open the file chooser");
      await form.$("textarea").setValue("林岚在雨夜书店中的全身参考，保留风衣与右手怀表。");
      assert.equal(await (await textButton("上传立绘", form)).isEnabled(), true);
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("upload_portrait")), false);
      return form;
    }, text: ["林岚", "thelittleprince.png", "更换图片"], last: async (scope) => scope.$('[role="switch"]'), cleanup: async (scope) => {
      const prompt = await scope.$("textarea");
      const draft = await prompt.getValue();
      await browser.keys("Escape");
      const confirmation = await dialog("未保存修改");
      await clickText("取消", confirmation);
      assert.equal(await prompt.getValue(), draft, "Cancel must preserve the selected file's upload description");
      await browser.keys("Escape");
      const discard = await dialog("未保存修改");
      await clickText("放弃草稿", discard);
      await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"]')]
        .some((item) => item.textContent?.includes("上传角色立绘"))));
      return { closeConfirmationPreservedDraft: true, discardClosedDialog: true };
    } },
    { id: "N03-portrait-preview", task: "独立预览立绘并用 Escape 返回原按钮", open: async () => {
      const scope = await portraits();
      const card = await scope.$('button[title="管理人物三视图拆解"]').$("..");
      return previewRoundTrip(await ariaButton("图片预览", card));
    }, check: async (scope) => {
      const stage = await scope.$(".a4n-image-lightbox-stage");
      assert.equal(await stage.getAttribute("title"), "使用缩放按钮调整图片，放大后可拖动查看");
      const previewHint = await stage.getAttribute("title");
      const scale = scope.$(".a4n-image-lightbox-toolbar > span");
      assert.equal(await scale.getText(), "100%");
      const zoomIn = await ariaButton("放大图片", scope);
      for (let step = 0; step < 3; step += 1) await press(zoomIn);
      assert.equal(await scale.getText(), "175%", "Zoom controls must update the visible image scale");
      return { separatePreview: true, escapeFocusRestored: true, previewHint, zoomScale: await scale.getText(), previewSource: await scope.$("img").getAttribute("src") };
    }, last: async (scope) => {
      const reset = await ariaButton("重置图片视图", scope);
      await press(reset);
      assert.equal(await scope.$(".a4n-image-lightbox-toolbar > span").getText(), "100%");
      return reset;
    } },
    { id: "N03-reference-selection", task: "图片中央选择参考图，独立预览保留选择并恢复焦点", open: async () => {
      const scope = await portraits();
      await press(await scope.$('button[aria-label="更多操作"]'));
      await menuItem("生成单个角色立绘");
      const form = await dialog("生成人物三视图拆解");
      await browser.waitUntil(async () => (await form.$$('button[title="选择参考图"]')).length >= 12, { timeout });
      return form;
    }, check: async (scope) => {
      const select = await scope.$('button[title="选择参考图"]');
      const picture = await select.$("img");
      assert.equal(await select.getAttribute("aria-pressed"), "false");
      await press(picture);
      assert.equal(await select.getAttribute("aria-pressed"), "true", "Image center must select the reference");
      assert.equal(await $(".a4n-image-lightbox-panel").isDisplayed(), false);
      const zoom = await ariaButton("图片预览", await select.$(".."));
      await press(zoom);
      await lightbox();
      assert.equal(await select.getAttribute("aria-pressed"), "true", "Preview must preserve the selection");
      await closeDialog();
      await assertFocus(zoom);
      assert.equal(await select.getAttribute("aria-pressed"), "true");
      await press(picture);
      assert.equal(await select.getAttribute("aria-pressed"), "false", "Image center must also deselect");
      await press(picture);
      assert.equal(await select.getAttribute("aria-pressed"), "true");
      return { referenceImages: (await scope.$$('button[aria-pressed]')).length, imageCenterToggles: true, previewPreservesSelection: true, escapeFocusRestored: true };
    }, last: async (scope) => {
      const selectedReferenceCount = () => scope.execute((root) =>
        [...root.querySelectorAll('button[title="取消选择参考图"]')]
          .filter((button) => button.getAttribute("aria-pressed") === "true").length,
      );
      while (await selectedReferenceCount() < 4) {
        await press(await scope.$('button[title="选择参考图"]'));
      }
      const selectedImages = await scope.$$('button[title="取消选择参考图"]');
      assert.equal(selectedImages.length, 4);
      const lastImage = selectedImages.at(-1);
      assert.equal((await reach(lastImage)).reached, true);
      const count = await scope.$('[role="status"]');
      assert.equal(await count.getText(), "已选 4 张，最多 4 张");
      const referenceSection = await scope.$('.//section[.//h3[normalize-space(.)="参考图"]]');
      const upload = await referenceSection.$('input[type="file"]');
      const uploadCount = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "upload_image").length);
      await upload.execute((node) => {
        node.classList.remove("hidden");
        node.style.display = "block";
        node.style.opacity = "1";
      });
      await upload.setValue(fileURLToPath(new URL("../../resources/thelittleprince.png", import.meta.url)));
      await waitText($("body"), "最多选择 4 张参考图，请先取消一张再选择或上传");
      assert.equal(await selectedReferenceCount(), 4, "Uploading at the reference limit must preserve every selected image");
      assert.equal(await browser.execute((before) => window.__uiAudit.requests.filter((item) => item.command === "upload_image").length, uploadCount), uploadCount, "An upload at the reference limit must not reach the API");
      const visible = await wheelTo(count);
      assert.ok(visible.reached && visible.wheelSteps === 0, "Selected-reference count must remain visible at the end of the gallery");
      return lastImage;
    }, cleanup: async (scope) => {
      const selected = await scope.$('button[aria-pressed="true"]');
      await browser.keys("Escape");
      const confirmation = await dialog("未保存修改");
      await clickText("取消", confirmation);
      assert.equal(await selected.getAttribute("aria-pressed"), "true", "Cancel must preserve the selected reference");
      await browser.keys("Escape");
      const discard = await dialog("未保存修改");
      await clickText("放弃草稿", discard);
      await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"]')]
        .some((item) => item.textContent?.includes("生成人物三视图拆解"))));
      return { closeConfirmationPreservedSelection: true, discardClosedDialog: true };
    } },
    ...[
      ["behavior", "行为", 6], ["implicit", "复审", 3], ["event", "属性", 6],
      ["snapshot", "快照", 3], ["deletion", "删除", 2],
    ].map(([id, label, minimum]) => ({
      id: "N03-manage-" + id, task: "阅读角色" + label + "管理记录",
      open: () => characterTab("管理", label),
      check: async (scope) => {
        await browser.waitUntil(async () => (await scope.$$("li")).length >= minimum, { timeout, timeoutMsg: "Populated " + label + " records missing" });
        if (id === "behavior") {
          const visibleRecord = await scope.$("li").getText();
          await browser.execute(() => window.__uiAudit.failNext.push({
            command: "list_behaviors",
            status: 503,
            message: "行为读取服务暂时不可用",
          }));
          await press(await textButton("刷新", scope));
          await browser.waitUntil(async () => (await scope.$('[role="alert"]').getText()).includes("行为读取服务暂时不可用"), { timeout });
          assert.equal(await scope.$("li").getText(), visibleRecord, "A failed refresh must identify the error while retaining the last successful behavior list");
        }
        return { records: (await scope.$$("li")).length, minimum };
      }, last: async (scope) => (await scope.$$("li")).at(-1),
    })),
    { id: "N03-snapshot-compare", task: "比较第一章到第三章的角色状态并核对变化", open: async () => {
      const scope = await characterTab("管理", "快照");
      const summary = await containing("比较与回滚", scope, "summary");
      await press(summary);
      const tools = await summary.$("..");
      const directionIcon = await tools.$('button[role="combobox"] + span svg.lucide-arrow-right');
      assert.equal(await directionIcon.isExisting(), true, "The comparison range must use the Lucide direction icon");
      assert.equal(await directionIcon.getAttribute("aria-hidden"), "true");
      assert.equal(await tools.$(".sr-only").getText(), "至", "The comparison direction must remain available to assistive technology");
      assert.deepEqual(await snapshot("N03-snapshot-compare", current.width, "range-controls", tools), []);
      const selects = await tools.$$('button[role="combobox"]');
      for (const [index, chapter] of [[0, 1], [1, 3]]) {
        await press(selects[index]);
        await press(await $('//*[@role="option" and normalize-space(.)=' + literal(`第 ${chapter} 章`) + ']'));
      }
      await clickText("生成差异", tools);
      await waitText(scope, "旧车站第三站台");
      const delta = await scope.$('.//section[.//div[normalize-space(.)="快照变化"]]');
      const deltaDirectionIcon = await delta.$("svg.lucide-arrow-right");
      assert.equal(await deltaDirectionIcon.isExisting(), true, "The compared chapter range must use the Lucide direction icon");
      assert.equal(await deltaDirectionIcon.getAttribute("aria-hidden"), "true");
      assert.equal(await delta.$(".sr-only").getText(), "至", "The compared chapter direction must remain available to assistive technology");
      assert.equal(await scope.$('[role="region"][tabindex="-1"]').isFocused(), true, "Snapshot comparison must reveal and focus its result");
      return scope;
    }, check: async () => {
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "diff_profile_chapters")?.body.input);
      assert.deepEqual(request, { profile_id: 1011, from_chapter: 1, to_chapter: 3 });
      return { comparedChapters: [1, 3] };
    }, last: async (scope) => scope.$(".//span[contains(normalize-space(.),'旧车站第三站台')]") },
    { id: "N03-snapshot-state", task: "从第一章快照读取并展开当章完整属性", open: async () => {
      const scope = await characterTab("管理", "快照");
      const row = await scope.$("li");
      await row.waitForDisplayed({ timeout });
      await clickText("状态", row);
      const summary = seed.byProfileChapter["1011"]["1"].state_summary;
      const detail = await scope.$(".//section[.//p[normalize-space(.)=" + literal(summary) + "]]");
      await detail.waitForDisplayed({ timeout });
      assert.equal(await scope.$('[role="region"][tabindex="-1"]').isFocused(), true, "Snapshot state must reveal and focus its result");
      await press(await detail.$("summary"));
      return detail;
    }, text: ["旧书店", "亲自核验父亲留下的线索"], check: async (scope) => {
      const counts = seed.byProfileChapter["1011"]["1"].attribute_counts;
      assert.equal((await scope.$$("dd")).length, counts.explicit + counts.implicit + counts.social);
      const firstAttribute = await wheelTo(await scope.$("dd"));
      assert.ok(firstAttribute.reached);
      return { chapter: 1, attributeCounts: counts, firstAttribute };
    }, last: async (scope) => (await scope.$$("dd")).at(-1) },
    { id: "N03-snapshot-detail", task: "从快照 hash 读取当章变更与行为证据", open: async () => {
      const scope = await characterTab("管理", "快照");
      const row = await scope.$("li");
      await row.waitForDisplayed({ timeout });
      const snapshotHash = seed.byProfile["1011"].list_snapshots[0].snapshot_hash;
      await browser.execute((hash) => {
        const snapshot = window.__uiAudit.fixtures.bySnapshot[hash];
        const existing = snapshot.data.changes;
        snapshot.data.changes = [...existing, ...Array.from({ length: 4 }, (_, index) => ({
          ...existing[0],
          id: 101200 + index,
          key: `additional_${index + 1}`,
          change_description: `补充变化记录 ${index + 1}`,
        }))];
      }, snapshotHash);
      await clickText("详情", row);
      const summary = seed.byProfileChapter["1011"]["1"].state_summary;
      const detail = await scope.$(".//section[.//p[normalize-space(.)=" + literal(summary) + "]]");
      await detail.waitForDisplayed({ timeout });
      assert.equal(await scope.$('[role="region"][tabindex="-1"]').isFocused(), true, "Snapshot details must reveal and focus their result");
      return detail;
    }, text: ["根据已写正文更新当前位置。", "观察并记录物件与空间细节。"], check: async (scope) => {
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "get_snapshot")?.body.input);
      assert.equal(request?.content_hash, seed.byProfile["1011"].list_snapshots[0].snapshot_hash);
      await press(await containing("展开其余 2 条", scope, "summary"));
      await waitText(scope, "补充变化记录 4");
      return { snapshotHash: request.content_hash, changes: 6, expandedRemainingChanges: 2, behaviors: 2 };
    }, last: async (scope) => (await scope.$$("section")).at(-1) },
    { id: "N04-object-detail", task: "核对旧怀表的用途、证据与图像动作", open: async () => {
      const scope = await project("物体"); await press(await scope.$('button[aria-label^="旧怀表 ·"]')); return dialog();
    }, text: ["九点十七分", "林岚"] },
    { id: "N05-scene-detail", task: "核对书店空间、规则和来源", open: async () => {
      const scope = await project("场景"); await press(await scope.$('button[aria-label^="旧书店 ·"]')); return dialog();
    }, text: ["前门", "后门", "铜铃"] },
    { id: "N06-character-relations", task: "通过可读角色列表查看林岚的关系", open: async (width) => {
      const scope = await project("关系");
      await press(await scope.$('aside button[title="林岚"]'));
      return width < 1024 ? dialog("全部角色") : scope.$("aside");
    }, text: ["林岚", "周衡"], check: async (scope) => {
      const relationships = await scope.$$('button[title="查看关系详情"]');
      assert.ok(relationships.length >= 2, "Selected character relationships must be readable");
      const directions = await scope.execute((root) => [...root.querySelectorAll('button[title="查看关系详情"]')].map((button) => ({
        icon: button.querySelector('svg.lucide-arrow-right[aria-hidden="true"]') != null,
        label: button.querySelector(".sr-only")?.textContent.trim(),
      })));
      assert.equal(directions.length, relationships.length);
      assert.ok(directions.every((direction) => direction.icon && direction.label === "指向"), "Relationship direction must use Lucide and remain available to screen readers");
      return { visibleRelationshipEntries: relationships.length, lucideDirections: directions.length, directionText: "指向" };
    }, last: async (scope) => (await scope.$$('button[title="查看关系详情"]')).at(-1) },
    { id: "N06-relationship-detail", task: "从角色列表打开一条关系的完整说明", open: async (width) => {
      const scope = await project("关系");
      await press(await scope.$('aside button[title="林岚"]'));
      const list = width < 1024 ? await dialog("全部角色") : await scope.$("aside");
      await press(await list.$('button[title="查看关系详情"]'));
      return dialog("林岚 / 周衡");
    }, text: ["关系描述"] },
    { id: "N06-reset-layout", task: "拖动关系节点后恢复自动布局，保留全部连线", open: () => project("关系"), check: async (scope, width) => {
      const initialGraph = await relationshipGraph(scope, "beforeDrag");
      const node = await scope.$('.react-flow__node[aria-label^="角色 林岚，"]');
      const reachable = await wheelTo(node);
      assert.ok(reachable.reached, "The relationship node must be pointer-reachable before dragging");
      const position = () => node.execute((element) => {
        const rect = element.getBoundingClientRect();
        const cssTransform = getComputedStyle(element).transform;
        const transform = new DOMMatrixReadOnly(cssTransform);
        const pointerX = rect.x + rect.width / 2, pointerY = rect.y + rect.height / 2;
        const hit = document.elementFromPoint(pointerX, pointerY);
        return {
          x: transform.m41, y: transform.m42, pointerX, pointerY, cssTransform,
          rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
          nodeClass: element.getAttribute("class"),
          hit: hit ? { tag: hit.tagName, class: hit.getAttribute("class"), label: hit.getAttribute("aria-label"), nodeId: hit.closest(".react-flow__node")?.getAttribute("data-id") } : null,
        };
      });
      const initialPosition = await position();
      const deltaX = initialPosition.pointerX + 48 < width - 16 ? 48 : -48;
      current.evidence = { initialGraph, initialPosition, reachable };
      await browser.execute(() => {
        const audit = window.__uiAudit;
        audit.relationshipPointerEvents = [];
        const types = ["pointerdown", "pointermove", "pointerup", "mousedown", "mousemove", "mouseup", "dragstart", "dragend"];
        const record = (event) => {
          const entry = {
            type: event.type, x: event.clientX, y: event.clientY, button: event.button, buttons: event.buttons,
            trusted: event.isTrusted, defaultPrevented: event.defaultPrevented,
            tag: event.target?.tagName, class: event.target?.getAttribute?.("class"),
            nodeId: event.target?.closest?.(".react-flow__node")?.getAttribute("data-id"),
          };
          audit.relationshipPointerEvents.push(entry);
          queueMicrotask(() => { entry.defaultPrevented = event.defaultPrevented; });
        };
        types.forEach((type) => document.addEventListener(type, record, true));
        audit.stopRelationshipPointerCapture = () => types.forEach((type) => document.removeEventListener(type, record, true));
      });
      try {
        await browser.performActions([{ type: "pointer", id: "relationship-drag", parameters: { pointerType: "mouse" }, actions: [
          { type: "pointerMove", origin: "viewport", x: Math.round(initialPosition.pointerX), y: Math.round(initialPosition.pointerY) },
          { type: "pointerDown", button: 0 },
          { type: "pause", duration: 100 },
          ...Array.from({ length: 6 }, (_, index) => ({
            type: "pointerMove", origin: "viewport", duration: 80,
            x: Math.round(initialPosition.pointerX + deltaX * (index + 1) / 6),
            y: Math.round(initialPosition.pointerY - 24 * (index + 1) / 6),
          })),
          { type: "pointerUp", button: 0 },
        ] }]);
      } finally {
        await browser.releaseActions();
        const pointerEvents = await browser.execute(() => {
          window.__uiAudit.stopRelationshipPointerCapture();
          return window.__uiAudit.relationshipPointerEvents;
        });
        current.evidence = { ...current.evidence, pointerEvents };
      }
      const draggedPosition = await position();
      current.evidence = { ...current.evidence, draggedPosition };
      const draggedGraph = await relationshipGraph(scope, "afterDrag");
      const evidence = { ...current.evidence, initialGraph, draggedGraph, initialPosition, draggedPosition };
      current.evidence = evidence;
      assert.ok(Math.hypot(draggedPosition.x - initialPosition.x, draggedPosition.y - initialPosition.y) > 10, "The native drag must actually move the node: " + JSON.stringify({ initialPosition, draggedPosition }));
      assert.deepEqual(await snapshot("N06-reset-layout", width, "dragged", scope, evidence), []);
      const graphFrame = await scope.$(".react-flow").$("..");
      await press(await graphFrame.$('button[aria-label="更多操作"]'));
      await menuItem("恢复自动布局");
      const restoredGraph = await relationshipGraph(scope, "afterRestore");
      let restoredPosition;
      await browser.waitUntil(async () => {
        restoredPosition = await position();
        current.evidence = { ...evidence, restoredGraph, restoredPosition };
        return Math.abs(restoredPosition.x - initialPosition.x) <= 1 && Math.abs(restoredPosition.y - initialPosition.y) <= 1;
      }, { timeout, timeoutMsg: "Restore must return the dragged node to its original automatic layout position" });
      assert.equal(await graphFrame.$('button[aria-label="更多操作"]').isExisting(), false, "Restored layout must clear the fixed-position action");
      return { ...evidence, restoredGraph, restoredPosition };
    } },
    { id: "N07-foreshadow-index", task: "主动展开并阅读八条既有伏笔", open: async () => {
      const scope = await project("事件");
      await press(await containing("伏笔", scope, "summary"));
      return scope.$(".content-main > details");
    }, text: ["九点十七分"], check: async (scope) => {
      const count = (await scope.$$('button[aria-label^="编辑"]')).length;
      assert.equal(count, seed.commands.list_foreshadowing.length);
      return { foreshadowingRecords: count };
    }, last: async (scope) => (await scope.$$('button[aria-label^="编辑"]')).at(-1) },
    { id: "N07-foreshadow-editor", task: "读取既有伏笔状态、章号与关联人物", open: async () => {
      const scope = await project("事件");
      await press(await containing("伏笔", scope, "summary"));
      await press(await scope.$('button[aria-label^="编辑"]')); return dialog();
    }, check: async (scope) => {
      assert.ok((await scope.$("textarea").getValue()).includes("九点十七分"));
      return { existingForeshadowing: true };
    }, last: async (scope) => (await scope.$$("button")).at(-1) },
    { id: "N08-chapter-outlines", task: "查看九章大纲及末章入口", open: async () => {
      const scope = await project("大纲"); await clickText("章节大纲", scope); return scope;
    }, text: ["雨夜来信", "有人记得出发的地方"], check: async (scope) => {
      assert.equal((await scope.$$(".chapter-outline-open-indicator")).length, seed.commands.list_chapter_outlines.length);
      return { visibleEntryIndicators: seed.commands.list_chapter_outlines.length };
    }, last: async (scope) => scope.$('button[aria-label*="有人记得出发的地方"]') },
    { id: "N08-part-detail", task: "阅读全文分部并发现编辑动作", open: async () => {
      const scope = await project("大纲"); await clickText("分部大纲", scope);
      await press(await scope.$('button[aria-label*="夜雨中的旧书店"]')); return dialog();
    }, text: ["周衡保存了许多迟来的线索"], last: async (scope) => scope.$(".whitespace-pre-wrap") },
    { id: "N08-empty-chapter-settings", task: "没有章纲时核对生成范围与数量标签", open: async () => {
      await home();
      await browser.execute(() => {
        const outlines = window.__uiAudit.fixtures.list_chapter_outlines;
        if (!Array.isArray(outlines)) throw new Error("Chapter outline fixture is missing");
        window.__uiAudit.fixtures.list_chapter_outlines = [];
      });
      const row = await rootRow();
      await press(await row.$('button[aria-label="更多操作"]'));
      await menuItem("查看档案");
      await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/novel/1", { timeout });
      const page = await $(pageSelector);
      await page.waitForDisplayed({ timeout });
      await clickText("大纲", await $('nav[role="tablist"]'));
      await clickText("章节大纲", page);
      const details = await page.$("#outline-chapters-panel details");
      await details.waitForDisplayed({ timeout });
      await browser.waitUntil(async () => details.execute(element => element.open), { timeout });
      return details;
    }, text: ["章纲范围设置", "设置本次生成章数"], check: async (scope) => {
      const summary = await scope.$("summary").getText();
      const countLabel = await scope.$("label").getText();
      const action = await textButton("生成章纲", scope);
      const preview = await textButton("查看生成请求", scope);
      assert.equal(await scope.execute(element => element.open), true);
      assert(summary.includes("设置本次生成章数"));
      assert(countLabel.includes("本次生成章数"));
      assert.equal(await action.isEnabled(), true);
      assert.equal(await preview.isDisplayed(), true);
      assert.equal(await preview.isEnabled(), true);
      return { emptyOutlines: true, summary, countLabel, actionEnabled: true, previewVisible: true };
    }, last: async (scope) => textButton("生成章纲", scope) },
    { id: "N08-create-part", task: "检查手工录入分部的范围和正文输入", open: async () => {
      const scope = await project("大纲"); await clickText("分部大纲", scope); await clickText("直接录入", scope); return dialog();
    }, check: async (scope) => {
      assert.ok((await scope.$$("input")).length >= 3); assert.equal(await scope.$("textarea").isExisting(), true);
      return { manualPartInputs: true };
    }, last: async (scope) => (await scope.$$("button")).at(-1) },
    ...[["part", "分部大纲", "追加分部", 2], ["chapter", "章节大纲", "追加章纲", 1]].map(([kind, tab, action, count]) => ({
      id: "N08-append-" + kind, task: "展开" + tab + "追加范围并保留既有大纲",
      open: async () => {
        const page = await project("大纲"); await clickText(tab, page);
        const panelId = kind === "part" ? "outline-parts-panel" : "outline-chapters-panel";
        const details = await page.$(`#${panelId} details`);
        const summary = await details.$("summary");
        assert.equal(await details.execute(element => element.open), false);
        assert.equal(await summary.$("svg").isExisting(), true, "Collapsed settings need a direction cue");
        assert.deepEqual(await snapshot(current.id, current.width, "collapsed", details), []);
        await press(summary);
        await browser.waitUntil(async () => details.execute(element => element.open), { timeout });
        return details;
      }, check: async (scope) => {
        const fields = await scope.$$('input[type="number"]');
        assert.equal(fields.length, count);
        for (const field of fields) assert.ok((await field.getValue()).length > 0);
        const preview = await textButton("查看追加请求", scope);
        assert.ok(await (await textButton(action, scope)).isEnabled());
        assert.equal(await preview.isDisplayed(), true);
        assert.equal(await preview.isEnabled(), true);
        return { numericFields: count, generated: false, previewVisible: true };
      }, last: async (scope) => textButton(action, scope),
    })),
    { id: "N08-chapter-editor", task: "编辑已有章纲的标题和长摘要，保留保存与取消", open: async () => {
      const page = await project("大纲"); await clickText("章节大纲", page);
      await press(await page.$('button[aria-label="更多章纲操作"]'));
      await press(await containing("编辑章纲", await dialog()));
      const row = await page.$("li:has(textarea)");
      await row.$("textarea").waitForDisplayed({ timeout });
      assert.equal(await row.$("textarea").getValue(), seed.commands.list_chapter_outlines[0].summary);
      return row;
    }, text: ["保存", "取消"], last: async (scope) => textButton("保存", scope) },
    { id: "N08-create-chapter", task: "以新章号手工录入标题与章纲，检查末端保存", open: async () => {
      const page = await project("大纲"); await clickText("章节大纲", page); await clickText("直接录入", page);
      const form = await dialog();
      assert.equal(await form.$('input[type="number"]').getValue(), "10");
      await form.$('input[type="text"]').setValue("归来后的新线索");
      await form.$("textarea").setValue("保留旧书店与车站已有事实，用新的核验行动推动后续章节。");
      return form;
    }, last: async (scope) => (await scope.$$("button")).at(-1) },
    { id: "N08-part-editor", task: "在原分部范围内编辑完整剧情，保留主题与取消入口", open: async () => {
      const page = await project("大纲"); await clickText("分部大纲", page);
      await press(await page.$('button[aria-label*="夜雨中的旧书店"]'));
      await clickText("编辑正式分部", await dialog());
      const form = await dialog("编辑第 1 部");
      assert.ok((await form.$("textarea").getValue()).includes("林岚"));
      return form;
    }, last: async (scope) => (await scope.$$("textarea")).at(-1) },
    { id: "N08-advanced", task: "发现进度、节奏和维护工具", open: async () => {
      const scope = await project("大纲"); await clickText("分部大纲", scope);
      const entry = await textButton("高级工具", scope);
      assert.equal(await entry.getAttribute("aria-label"), "打开大纲高级工具");
      await press(entry); return dialog();
    }, text: ["生成进度"], check: async (scope, width) => {
      const actions = [];
      for (const expected of [
        { title: "生成本部章纲", description: "根据选中分部的目标和章节范围，生成该部对应的章节大纲。", button: "生成本部章纲", enabled: false, stage: "selected-part-action" },
        { title: "批量生成各部章纲", description: "为当前所有分部补齐对应的章节大纲。", button: "批量生成", enabled: true, stage: "all-parts-action" },
      ]) {
        const row = await scope.$('.//h5[normalize-space(.)=' + literal(expected.title) + ']/../..');
        const reachable = await reach(row);
        assert.ok(reachable.reached && reachable.rect.y >= reachable.clip.top && reachable.rect.bottom <= reachable.clip.bottom,
          "The complete action description and controls must be visible together: " + JSON.stringify(reachable));
        const description = await row.$("p").getText();
        assert.equal(description, expected.description, expected.title + " must describe its own generation scope");
        const action = await textButton(expected.button, row);
        assert.equal(await action.isEnabled(), expected.enabled);
        const buttonReachable = await wheelTo(action);
        assert.ok(buttonReachable.reached, "The matching action must be reachable in the same row");
        const evidence = { title: expected.title, description, action: expected.button, enabled: expected.enabled, reachable, buttonReachable };
        actions.push(evidence);
        current.evidence = { actions };
        assert.deepEqual(await snapshot("N08-advanced", width, expected.stage, scope, evidence), []);
      }
      return { actions };
    }, last: async (scope) => (await scope.$$("summary")).at(-1) },
    ...["概览", "来源", "管理"].map((tab, index) => ({
      id: "N09-image-detail-" + index, task: "点击图库图片中央并检查" + tab + "内页",
      open: async () => {
        const scope = await project("图像");
        const picture = await scope.$(index === 1
          ? 'button[title="管理图片"] img[src$="/image-40001.svg"]'
          : 'button[title="管理图片"] img');
        await picture.waitForDisplayed({ timeout });
        await press(picture);
        const detail = await dialog();
        await clickText(tab, detail);
        return detail;
      }, text: ["概览", "来源", "管理"], check: async (scope, width) => {
        assert.equal(await $(".a4n-image-lightbox-panel").isDisplayed(), false, "Image center must open management");
        if (index === 0) {
          const metadata = await reach(await scope.$("dl"));
          assert.ok(metadata.reached, "The overview metadata must be user-reachable");
          assert.equal(metadata.wheelSteps, 0, "The overview metadata must be visible without scrolling past an oversized preview");
          return { imageCenterOpenedManagement: true, selectedTab: tab, metadata };
        }
        if (index === 1) {
          const expected = seed.commands.list_images_by_projects.find((image) => image.id === 40001);
          const metadataText = (await Promise.all([...await scope.$$("dl")].map((list) => list.getText()))).join("\n");
          assert.ok(metadataText.includes(expected.metadata_json.generation.config_name), "The source tab must contain populated generation fields");
          assert.ok(metadataText.includes("来源章节") && metadataText.includes(`第 ${expected.metadata_json.source.chapter_number} 章`), "Source chapter must be readable without opening raw JSON");
          assert.equal((await scope.getText()).includes("没有可展示的图片详情。"), false);
          assert.deepEqual(await snapshot("N09-image-detail-1", width, "source-summary", scope, { metadataText }), []);
          await press(await scope.$("summary"));
          const rawRegion = await scope.$('[role="region"][aria-label="原始生成记录"]');
          const rawLayout = await browser.execute((region) => {
            const code = region.querySelector("pre");
            return {
              tabIndex: region.tabIndex,
              overflowX: getComputedStyle(region).overflowX,
              whiteSpace: getComputedStyle(code).whiteSpace,
              clientWidth: region.clientWidth,
              scrollWidth: region.scrollWidth,
              pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
            };
          }, rawRegion);
          assert.equal(rawLayout.tabIndex, 0, "Raw diagnostics must be keyboard-scrollable");
          assert.equal(rawLayout.overflowX, "auto");
          assert.equal(rawLayout.whiteSpace, "pre", "Paths and hashes must keep their JSON line structure");
          assert.equal(rawLayout.pageOverflow, 0, "The code region must contain long lines without widening the page");
          if (width === 390) assert.ok(rawLayout.scrollWidth > rawLayout.clientWidth, "Long diagnostic values must scroll inside the narrow code region");
          const actual = JSON.parse(await scope.$("pre").getText());
          assert.equal(actual.record.id, expected.id);
          assert.deepEqual(actual.metadata_json.source, expected.metadata_json.source);
          return { imageCenterOpenedManagement: true, selectedTab: tab, imageId: actual.record.id, source: actual.metadata_json.source, metadataText, rawLayout };
        }
        return { imageCenterOpenedManagement: true, selectedTab: tab };
      },
      last: index === 0 ? async (scope) => scope.$(".whitespace-pre-wrap")
        : index === 1 ? async (scope) => scope.$("pre") : undefined,
    })),
    { id: "N09-image-preview", task: "独立放大图库图像并按 Escape 回到预览按钮", open: async () => {
      const scope = await project("图像");
      const main = await scope.$('button[title="管理图片"]');
      await main.waitForDisplayed({ timeout });
      return previewRoundTrip(await ariaButton("图片预览", await main.$("..")));
    }, check: async (scope) => {
      const hint = await scope.$(".a4n-image-lightbox-stage").getAttribute("title");
      assert(hint.includes("下方按钮") && !hint.includes("滚轮"), "Zoom guidance must match controls available on touch screens");
      return { separatePreview: true, escapeFocusRestored: true, previewSource: await scope.$("img").getAttribute("src"), hint };
    } },
    ...["project", "chapter", "object"].map((mode) => ({
      id: "N09-generate-" + mode,
      task: mode === "object" ? "从完整物体档案选择素材并检查模型请求" : "填写" + (mode === "chapter" ? "本章" : "项目") + "图像提示词并检查模型请求",
      open: async () => {
        const scope = mode === "chapter" ? await writing("图像") : await project("图像");
        await clickText("生成图片", scope);
        const form = await dialog(mode === "chapter" ? "生成章节图像" : "生成项目图像");
        if (mode === "object") {
          await press(await form.$("summary"));
          await press(await form.$("#object-image-profile"));
          assert.equal((await $$('[role="option"]')).length, seed.commands.list_object_profiles.length + 1);
          await press(await $('//*[@role="option" and normalize-space(.)="旧怀表"]'));
        } else {
          assert.equal(await (await textButton("开始生成", form)).isEnabled(), false);
          await form.$("textarea").setValue(seed.commands.preview_scene_image_request.steps[0].user_prompt);
        }
        return form;
      }, check: async (scope, width) => {
        const id = "N09-generate-" + mode;
        const section = await scope.$(mode === "object" ? "details[open] section" : "section");
        const action = await textButton(mode === "object" ? "生成素材图" : "开始生成", section);
        await action.waitForEnabled({ timeout });
        assert.ok((await wheelTo(action)).reached);
        assert.deepEqual(await snapshot(id, width, "filled-form", scope), []);
        await press(await section.$('button[aria-label="更多操作"]'));
        await menuItem("查看模型请求");
        const command = mode === "object" ? "preview_object_profile_image_request" : "preview_scene_image_request";
        const preview = await dialog(seed.commands[command].title);
        const expected = seed.commands[command].steps[0];
        await waitText(preview, expected.user_prompt);
        const input = await browser.execute((name) => window.__uiAudit.requests.findLast((item) => item.command === name)?.body.input, command);
        assert.equal(input.chapter_id, mode === "chapter" ? 1 : null);
        assert.ok(input.width > 0 && input.height > 0);
        if (mode === "object") assert.equal(input.object_profile_id, 2011);
        else {
          assert.equal(input.project_id, 1);
          assert.equal(input.prompt, expected.user_prompt);
        }
        assert.deepEqual(await snapshot(id, width, "request-preview", preview, { input }), []);
        await closeDialog();
        if (mode === "object") assert.equal(await scope.$("#object-image-profile").getText(), "旧怀表");
        else assert.equal(await scope.$("textarea").getValue(), expected.user_prompt);
        return { chapterId: input.chapter_id, objectId: input.object_profile_id ?? null, requestPreviewAndDraftPreserved: true };
      }, last: async (scope) => textButton(mode === "object" ? "生成素材图" : "开始生成", scope),
    })),
    { id: "N10-review-detail", task: "展开候选的来源与结构化审核内容", open: async () => {
      const scope = await project("导入/归纳");
      await press(await scope.$('button[aria-label="查看 01-旧书店与车站调查补充材料.md 的审核详情"]'));
      const detail = await scope.$("aside");
      await press(await detail.$("summary"));
      return detail;
    }, check: async (scope) => {
      const value = await scope.$("textarea").getValue();
      assert.ok(value.includes('"chapter_title"') && value.includes('"content"'));
      const fieldId = await (await scope.$("textarea")).getAttribute("id");
      assert.ok(fieldId, "The analysis structure field must have an ID");
      assert.equal(await (await scope.$("label[for]")).getAttribute("for"), fieldId);
      const action = await textButton("保存并接受", scope);
      assert.equal(await action.isEnabled(), false);
      await waitText(scope, "修改上方审核内容后，才能保存并接受。");
      await waitText(scope, "修改分析结构后即可保存并接受。");
      const disabledReasons = await scope.$$('p[role="status"]');
      const reasonTexts = await Promise.all(disabledReasons.map((reason) => reason.getText()));
      return { reviewJsonLength: value.length, labelLinked: true, disabledReasons: reasonTexts };
    }, last: async (scope) => (await scope.$$("button")).at(-1) },
    { id: "N10-dedup", task: "对照已有章节检查重复，阅读结论与完整诊断且保留输入", open: async (width) => {
      const page = await project("导入/归纳");
      const summary = await containing("重复内容检查", page, "summary"); await press(summary);
      const scope = await summary.$("..");
      const content = seed.commands.list_chapters[0].content;
      // Explicit read-response fixture; this UI check does not run the SimHash implementation.
      await browser.execute(() => { window.__uiAudit.fixtures.check_import_dedup = {
        content_hash: "0000000000000000", is_duplicate: true, matched_chapter_id: 1, closest_distance: 0,
      }; });
      await scope.$("textarea").setValue(content);
      assert.ok((await scope.$("textarea").getSize()).height >= 140);
      assert.deepEqual(await snapshot("N10-dedup", width, "input", scope), []);
      await clickText("检查", scope);
      await press(await containing("查看去重结果", scope, "summary"));
      await waitText(scope, "检测到重复内容");
      return scope;
    }, check: async (scope) => {
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "check_import_dedup")?.body.input);
      assert.deepEqual(request, { project_id: 1, content: seed.commands.list_chapters[0].content.trim() });
      assert.equal(await scope.$("textarea").getValue(), seed.commands.list_chapters[0].content);
      assert.equal(await scope.$("dd").getText(), "是");
      await press(await containing("诊断信息", scope, "summary"));
      assert.deepEqual(JSON.parse(await scope.$("pre").getText()), { content_hash: "0000000000000000", is_duplicate: true, matched_chapter_id: 1, closest_distance: 0 });
      const opener = await textButton("查看匹配章节", scope);
      await browser.execute(() => { window.__uiAudit.deferred.push("get_chapter"); });
      await press(opener);
      await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "get_chapter")));
      assert.equal(await opener.getAttribute("aria-busy"), "true");
      await browser.execute(() => {
        const pending = window.__uiAudit.pending.findLast((item) => item.command === "get_chapter");
        window.__uiAudit.deferred = window.__uiAudit.deferred.filter((item) => item !== "get_chapter");
        pending.resolve(Response.json(window.__uiAudit.fixtures.byChapter[1].get_chapter));
      });
      const matched = await dialog();
      const dialogOpacity = await browser.execute(() =>
        getComputedStyle(document.querySelector('[role="dialog"]')).getPropertyValue("--a4n-dialog-opacity").trim(),
      );
      assert.equal(dialogOpacity, "1", "The long chapter preview must use an opaque surface");
      const chapterTitle = await matched.$("h3").getText();
      assert.ok(chapterTitle.includes("第 1 章") && chapterTitle.includes("雨夜来信"), "The read-only match preview must identify the matched chapter");
      const body = await matched.$("article");
      assert.equal(await body.getText(), seed.commands.list_chapters[0].content);
      assert.ok((await reach(body)).reached);
      assert.deepEqual(await snapshot("N10-dedup", current.width, "matched-chapter", matched), []);
      await closeDialog();
      await assertFocus(opener);
      assert.equal(await scope.$("textarea").getValue(), seed.commands.list_chapters[0].content);
      return { inputPreserved: true, readOnlyCheck: true, matchedChapterRead: true };
    }, last: async (scope) => scope.$("pre") },
    ...[["blueprint", "故事蓝图", "归纳正式蓝图"], ["outline", "章节大纲", "生成章节大纲候选"], ["part", "分部大纲", "生成分部大纲候选"]].map(([kind, target, action]) => ({
      id: "N10-consolidate-" + kind, task: "检查" + target + "归纳的上下文、人工补充和写入说明",
      open: async () => {
        const page = await project("导入/归纳");
        const summary = await containing("项目资料归纳", page, "summary"); await press(summary);
        const scope = await summary.$("..");
        await press(await scope.$('button[role="combobox"]'));
        await press(await $('//*[@role="option" and normalize-space(.)=' + literal(target) + ']'));
        const context = await scope.$('button[role="switch"]');
        if (await context.getAttribute("aria-checked") === "true") await press(context);
        await scope.$("textarea").setValue(seed.commands.list_chapters[0].content);
        return scope;
      }, check: async (scope) => {
        assert.ok((await scope.$("textarea").getSize()).height >= 140);
        assert.equal(await scope.$('button[role="switch"]').getAttribute("aria-checked"), "false");
        assert.ok(await (await textButton(action, scope)).isEnabled());
        await waitText(scope, kind === "blueprint" ? "直接成为正式蓝图" : "确认后才更新正式大纲");
        if (kind === "blueprint") {
          await waitText(scope, "依据下方样本文本归纳蓝图");
          const context = await scope.$('button[role="switch"]');
          await press(context);
          await waitText(scope, "依据当前正文、简介、大纲和正式资料归纳蓝图");
          await press(context);
          await waitText(scope, "依据下方样本文本归纳蓝图");
          assert.equal(await scope.$("textarea").getValue(), seed.commands.list_chapters[0].content);
        }
        return { target, contextFromEnteredContent: true, generationSubmitted: false };
      }, last: async (scope) => textButton(action, scope),
    })),
    { id: "N11-import", task: "发现手动导入章节的输入与确认动作", open: async () => {
      const scope = await project("章节"); await clickText("导入章节", scope); return dialog();
    }, text: ["确认导入"], check: async (scope) => {
      const semantics = await scope.execute((dialogNode) => {
        const number = dialogNode.querySelector('input[type="number"]');
        const body = dialogNode.querySelector("textarea");
        const confirm = [...dialogNode.querySelectorAll("button")]
          .find((button) => button.textContent?.trim() === "确认导入");
        return {
          numberLabels: [...(number?.labels ?? [])].map((label) => label.textContent?.trim()),
          bodyLabels: [...(body?.labels ?? [])].map((label) => label.textContent?.trim()),
          confirmDisabled: confirm?.disabled ?? null,
        };
      });
      assert.deepEqual(semantics.numberLabels, ["章节序号"]);
      assert.deepEqual(semantics.bodyLabels, ["正文"]);
      assert.equal(semantics.confirmDisabled, true, "Empty chapter content must not be importable");
      return { chapterNumberLabel: semantics.numberLabels[0], bodyLabel: semantics.bodyLabels[0], emptySubmitDisabled: true };
    }, last: async (scope) => textButton("确认导入", scope), cleanup: async () => {
      await closeDialog();
      await assertFocus(await textButton("导入章节", await $(pageSelector)));
      return { closedWithEscape: true, focusReturned: true };
    } },
    { id: "W04-point", task: "选中首个图片点，核对名称、完整画面描述及素材操作", open: () => directorInspector(),
      text: ["名称", "画面描述", "固定端点不能拆分或删除"], check: async (scope) => {
        assert.equal(await scope.$('input:not([type="file"])').getValue(), directorPlan.points[0].label);
        assert.equal(await scope.$("textarea").getValue(), directorPlan.points[0].description);
        return { point: directorPlan.points[0].id, fullDescription: true };
      }, last: async (scope) => scope.$(".director-property-actions") },
    { id: "W04-point-upload", task: "选取本地图像并核对上传后的分镜预览与草稿保存", open: async () => {
      const scope = await directorInspector();
      await browser.execute((baseUrl) => {
        const fixtures = window.__uiAudit.fixtures;
        const current = fixtures.byProject[3].byChapter[1].get_chapter_visual_plan.draft;
        const image = {
          id: 990041, project_id: 3, chapter_id: 1, panel_id: null,
          file_path: `${baseUrl}/e2e/ui-page-audit/media/image-42.svg`,
          width: 640, height: 360, file_size: 0, image_type: "chapter_reference",
          prompt: current.plan.points[0].description, model: "ui-upload-fixture",
          ai_score: null, is_favorite: false, is_system_managed: false,
          metadata_json: null, created_at: "2026-10-13T00:00:00Z",
        };
        const plan = structuredClone(current.plan);
        plan.points = plan.points.map((point) => point.id === "p0" ? { ...point, image_id: image.id } : point);
        fixtures.upload_image = image;
        fixtures.save_chapter_visual_plan_draft = {
          ...structuredClone(current), plan,
          record: { ...current.record, revision: current.record.revision + 1, plan_sha256: "ui-upload-fixture-plan" },
        };
      }, browser.options.baseUrl);
      return scope;
    }, captureInitialBeforeCheck: true, check: async (scope) => {
      const file = await scope.$('input[type="file"]');
      assert.equal(await file.isEnabled(), true);
      await browser.execute(() => window.__uiAudit.deferred.push("upload_image"));
      await file.setValue(fileURLToPath(new URL("../../resources/thelittleprince.png", import.meta.url)));
      await browser.waitUntil(async () => browser.execute(() =>
        window.__uiAudit.pending.some((item) => item.command === "upload_image"),
      ), { timeout, timeoutMsg: "The selected local image must reach the upload request fixture" });
      const uploadLabel = await scope.$(".director-upload-button");
      const uploadStatus = {
        label: await uploadLabel.getText(),
        inputDisabled: !(await file.isEnabled()),
        spinnerVisible: await uploadLabel.$("svg.animate-spin").isDisplayed(),
      };
      assert.match(uploadStatus.label, /上传中/, "Users need visible feedback while the selected image is uploading");
      assert.equal(uploadStatus.inputDisabled, true);
      assert.equal(uploadStatus.spinnerVisible, true, "The upload action should visibly distinguish its busy state");
      assert.deepEqual(await snapshot("W04-point-upload", current.width, "uploading", scope, uploadStatus), []);
      await browser.execute(() => {
        window.__uiAudit.deferred = window.__uiAudit.deferred.filter((command) => command !== "upload_image");
        const pending = window.__uiAudit.pending.findLast((item) => item.command === "upload_image");
        pending.resolve(Response.json(window.__uiAudit.fixtures.upload_image));
      });
      await browser.waitUntil(async () => browser.execute(() =>
        window.__uiAudit.requests.some((item) => item.command === "save_chapter_visual_plan_draft")
        && document.querySelector(".director-frame-image")?.currentSrc.includes("image-42.svg"),
      ), { timeout, timeoutMsg: "The selected image must upload and become the point preview after the draft save" });
      const requests = await browser.execute(() => ({
        upload: window.__uiAudit.requests.findLast((item) => item.command === "upload_image")?.body.input,
        save: window.__uiAudit.requests.findLast((item) => item.command === "save_chapter_visual_plan_draft")?.body.input,
      }));
      assert.equal(requests.upload.project_id, 3);
      assert.equal(requests.upload.chapter_id, 1);
      assert.equal(requests.upload.image_type, "chapter_reference");
      assert.equal(requests.upload.file_name, "thelittleprince.png");
      assert.ok(requests.upload.data_base64.length > 1000, "The selected local image must be encoded in the upload request");
      assert.equal(requests.save.plan.points.find((point) => point.id === "p0").image_id, 990041);
      assert.equal(await scope.$('input[type="file"]').isEnabled(), true, "The file chooser must be available after upload completes");
      return { uploadCommand: "upload_image", fileName: requests.upload.file_name, imageType: requests.upload.image_type,
        selectedBytesEncoded: true, uploadingFeedback: uploadStatus, savedPointId: "p0", savedImageId: 990041, previewLoaded: true, actualServerWrite: false };
    }, last: async (scope) => scope.$(".director-point-media-actions") },
    { id: "W04-confirm-plan", task: "确认已保存的导演编排草稿并核对只读状态", open: async () => {
      const scope = await videoDirector();
      await browser.execute(() => {
        const fixture = window.__uiAudit.fixtures.byProject[3].byChapter[1];
        const draft = structuredClone(fixture.get_chapter_visual_plan.draft);
        fixture.validate_chapter_visual_plan = {
          resource: { kind: "chapter_visual_plan", id: String(draft.record.id), hash: draft.record.plan_sha256, label: "导演编排", revision: String(draft.record.revision) },
          validation: { mode: "confirmed", valid: true, diagnostics: [] },
        };
        fixture.confirm_chapter_visual_plan = {
          invalidated_project_ids: [],
          shooting_script_sync: null,
          visual_plan: {
            ...draft,
            record: { ...draft.record, status: "confirmed", confirmed_at: "2026-10-13T00:00:00Z", revision: draft.record.revision + 1, plan_version: draft.record.plan_version + 1 },
          },
        };
      });
      return scope;
    }, captureInitialBeforeCheck: true, check: async (scope) => {
      const confirm = await textButton("确认方案", scope);
      assert.equal(await confirm.isDisplayed(), true);
      assert.equal(await confirm.isEnabled(), true);
      await press(confirm);
      await waitText(scope, "方案已确认");
      const editNewVersion = await textButton("编辑新版本", scope);
      assert.equal(await editNewVersion.isDisplayed(), true, "Confirmed plans should leave edit mode and expose the existing new-version action");
      const commands = await browser.execute(() => window.__uiAudit.requests
        .filter((item) => ["validate_chapter_visual_plan", "confirm_chapter_visual_plan", "save_chapter_visual_plan_draft"].includes(item.command))
        .map((item) => ({ command: item.command, input: item.body.input })));
      assert.deepEqual(commands.map((item) => item.command), ["validate_chapter_visual_plan", "confirm_chapter_visual_plan"]);
      assert.equal(commands[0].input.mode, "confirmed");
      assert.equal(commands[0].input.project_id, 3);
      assert.equal(commands[1].input.project_id, 3);
      assert.equal(commands[1].input.record_id, commands[0].input.record_id);
      assert.equal(commands[1].input.expected_revision, commands[0].input.expected_revision);
      assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) =>
        ["start_image_generation", "start_video_generation"].includes(item.command))), false);
      return { confirmedStateVisible: true, editModeEnded: true, actions: commands.map((item) => item.command), generationStarted: false };
    }, last: async (scope) => textButton("编辑新版本", scope) },
    { id: "W04-segment", task: "选中镜头段，检查变化描述、场景绑定、尾帧和时间线操作", open: () => directorInspector("segment"),
      text: ["镜头名称", "变化描述", "场景方式"], check: async (scope) => {
        assert.equal(await scope.$("input").getValue(), directorPlan.segments[0].title);
        assert.equal(await scope.$("textarea").getValue(), directorPlan.segments[0].prompt);
        return { segment: directorPlan.segments[0].id, fullPrompt: true };
      }, last: async (scope) => textButton("与后一段合并", scope) },
    { id: "W04-source", task: "从导演工具打开本章完整原文并到达末句", open: async () => {
      await videoDirector(); await press(await ariaButton("原文")); return dialog("章节原文");
    }, check: async (scope) => {
      assert.equal(await scope.$(".director-source").getText(), seed.commands.list_chapters[0].content);
      return { chapterId: 1, characters: seed.commands.list_chapters[0].content.length };
    }, last: async (scope) => scope.$(".director-source") },
    { id: "W04-scenes", task: "从导演工具查阅全部场景和最后一个地点", open: async () => {
      await videoDirector(); await press(await ariaButton("场景库")); return dialog("本项目场景库");
    }, text: ["旧书店", "沿海邮路档案室"], last: async (scope) => scope.$('button[aria-label^="沿海邮路档案室 ·"]') },
    { id: "W04-media", task: "查看图片点的素材库，保留选作分镜与参考两个动作", open: async () => {
      await videoDirector(); await press(await ariaButton("素材库")); return dialog(`${directorPlan.points[0].label} · 选择分镜与参考图`);
    }, check: async (scope) => {
      await browser.waitUntil(async () => (await scope.$$(".director-media-grid article")).length >= 12, { timeout });
      const captions = await scope.$$(".director-media-grid article > p");
      assert.equal(captions.length, (await scope.$$(".director-media-grid article")).length);
      assert.equal(await captions[0].getText(), (await scope.$(".director-media-grid img").getAttribute("alt")).trim());
      assert.equal(await (await textButton("选作参考", scope)).isEnabled(), true);
      return { images: (await scope.$$(".director-media-grid article")).length };
    }, last: async (scope) => (await scope.$$(".director-media-grid article")).at(-1) },
    { id: "W04-candidates", task: "检查选中图片点的候选与当前绑定状态", open: async () => {
      const inspector = await directorInspector();
      await press(await containing("分镜候选", inspector));
      return dialog(`${directorPlan.points[0].label} · 分镜候选`);
    }, check: async (scope) => {
      const candidates = await scope.$$('.director-point-candidate');
      assert.ok(candidates.length > 0);
      const bound = await scope.$('button[aria-label="当前绑定候选图"]');
      assert.equal(await bound.getAttribute("aria-pressed"), "true");
      assert.equal(await bound.isEnabled(), false);
      return { candidates: candidates.length, currentImageBound: true };
    }, last: async (scope) => (await scope.$$(".director-point-candidate")).at(-1) },
    { id: "W04-frame", task: "查看四种画幅选择并保留现有比例", open: async () => {
      await videoDirector(); await press(await ariaButton("画幅")); return dialog("画幅");
    }, text: ["16:9 横屏", "9:16 竖屏", "1:1 方形", "4:3"], check: async (scope) => {
      const selected = await scope.$$('button[aria-pressed="true"]');
      assert.equal(selected.length, 1, "The current aspect ratio must have one selected choice");
      assert.equal(await selected[0].getText(), "16:9 横屏");
      return { currentAspectRatio: "16:9" };
    }, last: async (scope) => textButton("4:3", scope) },
    { id: "W04-duration", task: "打开完整时间调整，核对总时长和等比伸缩", open: async () => {
      await videoDirector(); await press(await ariaButton("调整整体时长")); return dialog("调整时长");
    }, check: async (scope) => {
      assert.equal(await scope.$('input[aria-label="整体时长（秒）"]').getValue(), "36");
      assert.equal(await scope.$('input[aria-label="等比伸缩全部时间点"]').getValue(), "100");
      return { seconds: 36, scalePercent: 100 };
    }, last: async (scope) => textButton("完成", scope) },
    { id: "W04-merge", task: "合并前完整对照两个原镜头并填写新描述", open: async () => {
      const inspector = await directorInspector("segment");
      await clickText("与后一段合并", inspector);
      const scope = await dialog("合并镜头");
      await scope.$("textarea").setValue("保留街口到橱窗的连续动作，怀表始终在林岚右手。");
      return scope;
    }, text: [directorPlan.segments[0].prompt, directorPlan.segments[1].prompt],
    last: async (scope) => textButton("合并镜头", scope) },
    { id: "W04-split", task: "在有段内图片点的编排中检查拆分前后描述和场景", open: async () => {
      await videoDirector();
      await press(await $('.director-point[aria-label*="段内稳定构图"]'));
      await clickText("从此处分镜", await selectedDirectorInspector("point"));
      const scope = await dialog("拆分镜头");
      const prompts = await scope.$$("textarea");
      assert.equal(await prompts[0].getValue(), directorPlan.segments[0].prompt);
      assert.equal(await prompts[1].getValue(), "");
      await prompts[1].setValue("从稳定构图继续，镜头跟随林岚走向橱窗。");
      return scope;
    }, text: ["前段描述", "后段描述", "分镜位置"], last: async (scope) => (await scope.$$('button[role="combobox"]')).at(-1) },
    { id: "W04-agent", task: "打开第一章导演讨论，核对独立线程并到达输入区", open: async () => {
      await videoDirector();
      await press(await ariaButton("导演 Agent"));
      const scope = await $('[aria-label="领域 Agent 控制台"]');
      await waitText(scope, "本轮停在编排讨论");
      return scope;
    }, check: async (scope) => {
      assert.equal((await scope.$$(".agent-message-list > *")).length, videoDirectorSnapshot.messages.length);
      const requested = await browser.execute(() => window.__uiAudit.commands);
      assert.ok(requested.includes(videoDirectorSnapshot.thread.id));
      const input = await scope.$("textarea");
      assert.ok((await reach(input)).reached);
      await input.setValue("先核对门口到柜台的视线，再讨论剪辑接点。");
      assert.equal(await (await ariaButton("发送给领域 Agent", scope)).isEnabled(), true);
      return { boundThread: videoDirectorSnapshot.thread.id, messages: videoDirectorSnapshot.messages.length, sendEnabled: true };
    }, last: async (scope) => scope.$("textarea") },
    { id: "W05-script-editor", task: "进入视频项目的完整拍摄剧本编辑并读到最后一个场景", open: async () => {
      const scope = await videoScreenplay();
      await clickText("进入编辑", scope);
      await openScriptScene(scope, 0);
      return scope;
    }, text: ["取消编辑"], check: async (scope) => {
      const fields = await scope.$$("textarea");
      assert.ok(fields.length > 6, "Structured editing must include real scene and shot fields");
      return { textareas: fields.length };
    }, last: async (scope) => {
      await press(await scope.$('details[class~="group/scene"] > summary'));
      return openScriptScene(scope, -1);
    } },
    { id: "W05-direction", task: "单独阅读视频项目拍摄剧本的导演意图和完整策略", open: async () => {
      const scope = await videoScreenplay();
      const summary = await containing("导演意图", scope, "summary");
      await press(summary);
      const detail = await summary.$("..");
      await press(await detail.$(":scope > details > summary"));
      return detail;
    }, text: ["林岚"], last: async (scope) => (await scope.$$(".text-book-text-main")).at(-1) },
    { id: "W06-search", task: "实际检索章节资料并到达最后一条结果", open: async () => {
      const panel = await writingPanel("资料检索");
      await panel.$("textarea").setValue("怀表、旧车票和第三站台之间的联系");
      await clickText("检索", panel);
      await browser.waitUntil(async () => (await panel.$$("li")).length === 8, { timeout });
      return panel;
    }, text: ["相关度"], check: async (scope, width) => {
      const layout = await scope.execute((panel) => {
        const rect = (element) => {
          const bounds = element?.getBoundingClientRect();
          return bounds ? { width: bounds.width, left: bounds.left, right: bounds.right } : null;
        };
        const chapterSelector = document.querySelector(".a4n-writing-chapter-toolbar [role=combobox], .a4n-writing-chapter-toolbar select");
        return {
          viewportWidth: innerWidth,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          chapterSidebar: rect(document.querySelector(".a4n-writing-sidebar")),
          chapterSelector: rect(chapterSelector),
          workspace: rect(document.querySelector(".a4n-writing-workspace")),
          searchPanel: rect(panel),
          searchPanelInDialog: Boolean(panel.closest(".a4n-modal-layer")),
        };
      });
      assert.equal(layout.overflow, 0, "The writing desk must not overflow horizontally");
      if (width > 1024) {
        assert.ok(layout.chapterSidebar?.width > 0, "Wide writing desks must retain the chapter sidebar");
        assert.equal(layout.chapterSelector, null, "The compact chapter selector is only needed below 1024px");
        assert.equal(layout.searchPanelInDialog, false, "Wide search remains a side rail");
      } else if (width >= 768) {
        assert.equal(layout.chapterSidebar, null, "Compact writing desks must give the editor its width back");
        assert.ok(layout.chapterSelector?.width > 0, "Compact writing desks must retain direct chapter selection");
        assert.ok(layout.workspace?.width > 500, "The editor must remain readable beside the search rail");
        assert.ok(layout.searchPanel?.width >= 180, "The search rail must remain available beside the editor");
        assert.equal(layout.searchPanelInDialog, false, "The tablet search panel remains in the workspace");
      } else {
        assert.equal(layout.chapterSidebar, null, "Phone layout must not render a desktop chapter sidebar");
        assert.ok(layout.chapterSelector?.width > 0, "Phone layout must retain direct chapter selection");
        assert.equal(layout.searchPanelInDialog, true, "Phone search must remain in the existing modal");
      }
      return { layout };
    }, last: async (scope) => (await scope.$$("li")).at(-1) },
    { id: "W07-assistant", task: "查看生成要求、字数范围和上下文入口", open: async () => {
      const panel = await writingPanel("写作助手");
      await clickText("生成", panel);
      await panel.$("textarea").setValue("保留林岚和周衡既有事实，用核对旧地图的动作推动下一幕。");
      await press(await containing("正文长度", panel, "summary"));
      return panel;
    }, text: ["最少字数", "最多字数"], last: async (scope) => (await scope.$$("summary")).at(-1) },
    { id: "W09-scenes", task: "阅读并展开已有章节场景", open: async () => {
      const panel = await writingPanel("写作助手");
      await press(await panel.$('button[aria-label="更多操作"]'));
      await menuItem("场景管理");
      const scope = await dialog("章节场景");
      await press(await scope.$('li button[aria-expanded="false"]'));
      assert.equal(await scope.$("li pre").getText(), seed.byChapter[1].list_chapter_scenes[0].content);
      return scope;
    }, text: ["观察与核验", "行动与回应"], check: async (scope, width) => {
      const prompt = await scope.$("#scene-generation-prompt");
      const layout = await prompt.execute((element) => ({
        height: element.getBoundingClientRect().height,
        clientHeight: element.clientHeight,
        contentHeight: element.scrollHeight,
      }));
      if (width < 640) {
        assert.ok(layout.height >= 128, "The phone prompt must expose enough lines to avoid nested scrolling");
        assert.ok(layout.contentHeight <= layout.clientHeight + 1, "The default phone prompt must not scroll inside its textarea");
      }
      return { prompt: layout };
    }, last: async (scope) => (await scope.$$("li")).at(-1) },
    { id: "W10-profiles", task: "在写作过程中打开角色档案工作台", open: async () => {
      await writing();
      const trigger = await $('button[title="打开角色档案工作台"]');
      if (await trigger.isDisplayed()) await press(trigger);
      else {
        await press(await $('.a4n-app-page-header button[aria-label="更多操作"]'));
        await menuItem("角色档案");
      }
      return dialog();
    }, text: ["林岚", "沈照"] },
  ];
  for (const spec of inner) {
    if (spec.id === "N01-lineage" || spec.id === "N10-dedup" || spec.id.startsWith("N10-consolidate-")) {
      for (const width of widths) it(`${spec.id}-${width} checks its complete workflow`, async () => matrix(spec, [width]));
    } else if (["W02-analysis", "W03-images", "W06-search", "W07-assistant", "W08-optimizer", "W09-scenes", "W10-profiles"].includes(spec.id)) {
      it(spec.id + " exercises its inner page", async () => matrix(spec, [1440, 1024, 900, 390]));
    } else it(spec.id + " exercises its inner page", async () => matrix(spec));
  }

  it("N11-import keeps the footer reachable on a short viewport", async () => {
    current = { id: "N11-import-short", task: "在短视口中确认导入操作并关闭弹窗", visited: [], plannedWidths: [390] };
    await browser.setWindowSize(390, 540);
    const page = await project("章节");
    const trigger = await textButton("导入章节", page);
    await press(trigger);
    const scope = await dialog();
    await browser.pause(220);
    const layout = await browser.execute(() => {
      const panel = document.querySelector('[role="dialog"]');
      const body = panel?.querySelector('[data-modal-body="1"]');
      const footer = body?.nextElementSibling;
      const textarea = panel?.querySelector("textarea");
      const confirm = [...(footer?.querySelectorAll("button") ?? [])]
        .find((button) => button.textContent?.trim() === "确认导入");
      const rect = (node) => {
        const bounds = node?.getBoundingClientRect();
        return bounds ? { top: bounds.top, bottom: bounds.bottom, height: bounds.height } : null;
      };
      const bodyBounds = body?.getBoundingClientRect();
      const textareaBounds = textarea?.getBoundingClientRect();
      return {
        viewportHeight: innerHeight,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        bodyClientHeight: body?.clientHeight ?? 0,
        bodyScrollHeight: body?.scrollHeight ?? 0,
        textareaVisibleHeight: bodyBounds && textareaBounds
          ? Math.max(0, Math.min(bodyBounds.bottom, textareaBounds.bottom) - Math.max(bodyBounds.top, textareaBounds.top))
          : 0,
        footer: rect(footer),
        confirm: rect(confirm),
      };
    });
    current.evidence = { layout };
    assert.equal(layout.overflow, 0);
    assert.ok(layout.bodyScrollHeight > layout.bodyClientHeight, "The form body must scroll independently on a short screen");
    assert.ok(layout.footer?.bottom <= layout.viewportHeight + 2, "The dialog footer must remain inside the viewport");
    assert.ok(Math.round(layout.confirm?.height ?? 0) >= 44, "The confirm action must retain a touch-sized target");
    assert.ok(layout.confirm.bottom <= layout.viewportHeight + 1, "The confirm action must remain fully visible");
    assert.ok(layout.textareaVisibleHeight >= 80, "The short viewport must expose enough of the body editor to signal that it continues below");
    assert.deepEqual(await snapshot("N11-import-short", 390, "short-screen", scope, { layout }), []);
    await closeDialog();
    await assertFocus(trigger);
    current.visited.push(390);
  });

  it("UX-698 closes scene and generation drafts after one confirmation", async () => {
    current = { id: "UX-698", task: "同时丢弃场景字段和下一场景提示词草稿", visited: [], plannedWidths: [390] };
    await browser.setWindowSize(390, height);
    const panel = await writingPanel("写作助手");
    await press(await panel.$('button[aria-label="更多操作"]'));
    await menuItem("场景管理");
    const scope = await dialog("章节场景");
    const prompt = await scope.$("#scene-generation-prompt");
    const initialPrompt = await prompt.getValue();
    await prompt.setValue(initialPrompt + "\n只用于本次场景生成的临时要求。");

    const row = await scope.$("li");
    const title = await row.$(".text-sm");
    const initialTitle = await title.getText();
    await press(await row.$('button[aria-haspopup="menu"]'));
    await menuItem("编辑场景");
    const dirtyTitle = initialTitle + " · 未保存";
    await row.$("input").setValue(dirtyTitle);

    const secondRow = await scope.$("li:nth-of-type(2)");
    await press(await secondRow.$('button[aria-haspopup="menu"]'));
    await menuItem("编辑场景");
    const switchConfirmation = await dialog("未保存修改");
    await clickText("取消", switchConfirmation);
    assert.equal(await row.$("input").getValue(), dirtyTitle, "Cancel must keep the original scene edit when switching targets");

    await press(await scope.$('button[title^="关闭"]'));

    const confirmation = await dialog("未保存修改");
    const openConfirmations = await browser.execute(() => [...document.querySelectorAll('[role="dialog"] h3')]
      .filter((heading) => heading.textContent?.trim() === "未保存修改").length);
    assert.equal(openConfirmations, 1);
    current.evidence = { dirtySceneTitle: true, dirtyGenerationPrompt: true, sceneTargetSwitchBlocked: true, openConfirmations };
    await snapshot(current.id, 390, "single-discard-confirmation", confirmation, current.evidence);
    await clickText("放弃草稿", confirmation);
    await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"]')]
      .some((element) => element.textContent?.includes("章节场景"))));
    await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"] h3')]
      .some((heading) => heading.textContent?.trim() === "未保存修改")));

    const reopenedPanel = await writingPanel("写作助手");
    await press(await reopenedPanel.$('button[aria-label="更多操作"]'));
    await menuItem("场景管理");
    const reopened = await dialog("章节场景");
    assert.equal(await reopened.$("#scene-generation-prompt").getValue(), initialPrompt);
    assert.equal(await reopened.$("li .text-sm").getText(), initialTitle);
    await press(await reopened.$('button[title^="关闭"]'));
    await browser.waitUntil(async () => !await browser.execute(() => [...document.querySelectorAll('[role="dialog"]')]
      .some((element) => element.textContent?.includes("章节场景"))));
    current.visited.push(390);
  });

  it("UX-B60 keeps an edited foreshadowing draft behind the project route guard", async () => {
    current = { id: "UX-B60-foreshadowing", task: "浏览器返回时保留未保存伏笔编辑", visited: [], plannedWidths: [1440], width: 1440 };
    await browser.setWindowSize(1440, height);
    const page = await project("事件");
    await press(await containing("伏笔", page, "summary"));
    await press(await page.$('button[aria-label^="编辑伏笔"]'));
    const form = await dialog();
    const description = await form.$("textarea");
    const draft = `${await description.getValue()}\n仅用于当前伏笔编辑的草稿。`;
    await description.setValue(draft);
    const originalUrl = await browser.getUrl();

    await browser.back();
    let confirmation = await dialog("未保存修改");
    await clickText("取消", confirmation);
    assert.equal(await browser.getUrl(), originalUrl, "Cancel must keep the current route");
    assert.equal(await description.getValue(), draft, "Cancel must keep the foreshadowing text");

    await browser.back();
    confirmation = await dialog("未保存修改");
    await clickText("放弃草稿", confirmation);
    await browser.waitUntil(async () => (await browser.getUrl()) !== originalUrl);
    current.visited.push(1440);
  });

  it("UX-B60 protects a custom beat map when leaving advanced outlines", async () => {
    current = { id: "UX-B60-beat-map", task: "浏览器返回时保护未应用的节拍映射 JSON", visited: [], plannedWidths: [1440], width: 1440 };
    await browser.setWindowSize(1440, height);
    const page = await project("大纲");
    await press(await containing("高级工具", page, "button"));
    const form = await dialog();
    await press(await containing("节拍映射", form, "summary"));
    const field = await form.$("textarea");
    assert((await field.getValue()).includes('"beats"'));
    const draft = `${await field.getValue()}\n `;
    await field.setValue(draft);
    const originalUrl = await browser.getUrl();

    await browser.back();
    let confirmation = await dialog("未保存修改");
    await clickText("取消", confirmation);
    assert.equal(await browser.getUrl(), originalUrl, "Cancel must keep advanced outlines open");
    assert.equal(await field.getValue(), draft, "Cancel must preserve the beat-map input");

    await browser.back();
    confirmation = await dialog("未保存修改");
    await clickText("放弃草稿", confirmation);
    await browser.waitUntil(async () => (await browser.getUrl()) !== originalUrl);
    current.visited.push(1440);
  });

  const inspirationPages = [
    { id: "I01-conversation", task: "阅读十轮灵感对话并到达最后一个建议", open: inspiration,
      text: ["三种路径都保留书店", "先核对地图", "先听完周衡", "先验证门外信号"],
      check: async (scope) => {
        const options = await scope.$$(".inspiration-option-card");
        assert.equal(options.length, 3);
        assert.ok((await Promise.all([...options].map((option) => option.isEnabled()))).every(Boolean));
        const composerInput = await scope.$('textarea[aria-label="输入灵感对话内容"]');
        const composer = await reach(composerInput);
        assert.ok(composer.reached, "Conversation composer must remain reachable");
        assert.equal(await composerInput.getAttribute("aria-describedby"), null, "The placeholder must not be repeated above the composer in its normal state");
        return { historyMessages: seed.commands.get_inspiration_history.length, currentOptions: options.length, composer };
      }, last: async (scope) => (await scope.$$(".inspiration-option-card")).at(-1),
    },
    { id: "I01-history", task: "用真实滚动返回长对话开头", open: inspiration, check: async (scope) => {
      const first = await scope.$(".//div[normalize-space(.)=" + literal(seed.commands.get_inspiration_history[0].content) + "]");
      const firstMessage = await wheelTo(first, "start");
      assert.ok(firstMessage.reached, "The first message must be user-scrollable");
      assert.ok(firstMessage.rect.y >= firstMessage.clip.top - 1, "The first message must start below the clipping boundary");
      assert.ok(firstMessage.rect.bottom <= firstMessage.clip.bottom + 1, "The first message must be fully visible inside the conversation viewport");
      return { firstMessage };
    } },
    { id: "I02-progress", task: "查看已确认故事信息与下一步建议", open: async (width) => {
      const scope = await inspiration();
      if (width >= 1024) return scope.$("aside");
      await openHeaderMenu(); await menuItem("创作进度"); return dialog("创作进度");
    }, text: ["主角：林岚离家七年后归来", "先核对地图、听周衡经历"], check: async (scope, width) => {
      assert.equal((await scope.$$("li")).length, 6);
      const dialogOpacity = width < 1024
        ? await scope.execute((node) => getComputedStyle(node).getPropertyValue("--a4n-dialog-opacity").trim())
        : null;
      if (width < 1024) assert.equal(dialogOpacity, "1", "The mobile progress dialog must have an opaque surface");
      return { populatedProgressLines: 6, dialogOpacity };
    } },
    { id: "I03-turn-preview", task: "输入补充要求并查看本轮模型请求", open: async () => {
      const scope = await inspiration();
      await scope.$('textarea[aria-label="输入灵感对话内容"]').setValue(seed.commands.preview_inspiration_request.steps[0].user_prompt);
      await press(await scope.$('footer button[aria-label="更多操作"]'));
      await menuItem("查看本轮模型请求");
      return dialog(seed.commands.preview_inspiration_request.title);
    }, check: (scope) => checkRequestPreview(scope, "preview_inspiration_request"), last: lastRequestPreview },
    { id: "I04-blueprint-preview", task: "从灵感顶栏检查蓝图生成请求", open: async () => {
      await inspiration(); await openHeaderMenu(); await menuItem("蓝图请求预览");
      return dialog(seed.commands.preview_generate_blueprint_request.title);
    }, check: (scope) => checkRequestPreview(scope, "preview_generate_blueprint_request"), last: lastRequestPreview },
  ];
  for (const spec of inspirationPages) it(spec.id + " uses the real inspiration entry", async () => matrix(spec));

  it("UX-1752 blocks leaving while a conversation message is being accepted", async () => {
    current = { id: "UX-1752", task: "Keep the submitted message protected until the start request settles" };
    const scope = await inspiration();
    const composer = await scope.$('textarea[aria-label="输入灵感对话内容"]');
    const message = "请比较两种人物动机。";
    await composer.setValue(message);
    await browser.execute(() => window.__uiAudit.deferred.push("start_inspiration_generation"));
    await press(await textButton("发送", scope));
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "start_inspiration_generation"),
    ), { timeout });

    await press(await ariaButton("返回上一页"));
    const guard = await dialog("未保存修改");
    assert.ok((await guard.getText()).includes("操作仍在进行"));
    const discard = await textButton("放弃草稿", guard);
    assert.equal(await discard.isEnabled(), false);
    assert.equal(new URL(await browser.getUrl()).pathname, "/inspiration/1");

    await respondToPending("start_inspiration_generation", {
      code: "unavailable", message: "Injected inspiration start failure", retryable: true, trace_id: null, details: {},
    }, 503);
    await browser.waitUntil(async () => (await composer.getValue()) === message, { timeout });
    await browser.waitUntil(async () => discard.isEnabled(), { timeout });
    await press(await textButton("取消", guard));
    assert.equal(new URL(await browser.getUrl()).pathname, "/inspiration/1");
  });

  it("UX-1753 preserves composer text when sending a suggested reply", async () => {
    current = { id: "UX-1753", task: "Keep an unrelated composer draft when a suggested reply is selected" };
    const scope = await inspiration();
    const composer = await scope.$('textarea[aria-label="输入灵感对话内容"]');
    const draft = "补充：保留当前叙事视角。";
    await composer.setValue(draft);
    await browser.execute(() => window.__uiAudit.deferred.push("start_inspiration_generation"));
    await press((await scope.$$(".inspiration-option-card"))[0]);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "start_inspiration_generation"),
    ), { timeout });
    assert.equal(await composer.getValue(), draft);
    await respondToPending("start_inspiration_generation", {
      code: "unavailable", message: "Injected inspiration start failure", retryable: true, trace_id: null, details: {},
    }, 503);
    await browser.waitUntil(async () => (await composer.getValue()) === draft, { timeout });
  });

  it("UX-1754 preserves newer typing when an earlier send request fails", async () => {
    current = { id: "UX-1754", task: "Keep text entered while a conversation start request is pending" };
    const scope = await inspiration();
    const composer = await scope.$('textarea[aria-label="输入灵感对话内容"]');
    await composer.setValue("先发送这一条。");
    await browser.execute(() => window.__uiAudit.deferred.push("start_inspiration_generation"));
    await press(await textButton("发送", scope));
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "start_inspiration_generation"),
    ), { timeout });

    const newerDraft = "请求期间新写的补充内容。";
    await composer.setValue(newerDraft);
    await respondToPending("start_inspiration_generation", {
      code: "unavailable", message: "Injected inspiration start failure", retryable: true, trace_id: null, details: {},
    }, 503);
    await browser.waitUntil(async () => (await composer.getValue()) === newerDraft, { timeout });
  });
});

describe("Novel page audit - Runtime blueprints and bound optimizer", function () {
  this.timeout(60000);
  const pages = [
    { id: "I05-confirm", task: "经过 canonical 生成任务完成事件进入蓝图确认", open: blueprintConfirmation,
      text: ["故事蓝图", "林岚离开故乡七年", "确认进入项目"],
      check: async () => {
        const commands = await browser.execute(() => [...window.__uiAudit.commands]);
        for (const command of ["start_blueprint_generation", "get_blueprint", "sync_blueprint_materials"]) {
          assert.ok(commands.includes(command), "Missing real workflow request: " + command);
        }
        return { terminalResult: runtimeSeed.generation.blueprint.summary.result_ref, replayOnly: true };
      }, last: async (scope) => scope.$(".whitespace-pre-wrap"),
    },
    { id: "I06-refine", task: "在确认阶段填写蓝图调整要求", open: blueprintRefinement, text: ["提交优化"],
      check: async (scope) => {
        assert.ok(await (await textButton("提交优化", scope)).isEnabled());
        return { populatedRefinement: await scope.$("textarea").getValue() };
      }, last: async (scope) => textButton("提交优化", scope),
    },
    { id: "I07-refine-preview", task: "查看已填写调整要求的模型请求", open: async () => {
      const refine = await blueprintRefinement();
      await press(await refine.$('button[aria-label="更多操作"]'));
      await menuItem("查看优化模型请求");
      return dialog(seed.commands.preview_refine_blueprint_request.title);
    }, check: (scope) => checkRequestPreview(scope, "preview_refine_blueprint_request"), last: lastRequestPreview },
    { id: "I08-display", task: "确认蓝图，观察过渡页并到达项目详情", open: async () => {
      const scope = await blueprintConfirmation();
      await clickText("确认进入项目", scope);
      await waitText(scope, "蓝图已确认");
      return scope;
    }, text: ["正在打开项目详情"], last: async () => {
      const captured = report.at(-1);
      assert.equal(captured.url, "/inspiration/1", "The initial capture must show the display stage before navigation");
      assert.ok(captured.text.includes("蓝图已确认"));
      await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/novel/1", { timeout });
      const heading = await $('.a4n-app-page-header h1');
      await heading.waitForDisplayed({ timeout });
      assert.equal((await heading.getText()).trim(), rootTitle);
      return heading;
    } },
    { id: "I09-refine-failure", task: "调整提交返回 503 后继续编辑保留的输入", open: async (width) => {
      const form = await blueprintRefinement();
      const focus = await form.$("textarea").getValue();
      await browser.execute(() => window.__uiAudit.deferred.push("start_blueprint_refinement"));
      await clickText("提交优化", form);
      await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "start_blueprint_refinement")), { timeout });
      const request = await browser.execute(() => window.__uiAudit.pending.find((item) => item.command === "start_blueprint_refinement").body);
      assert.equal(request.project_id, 1);
      assert.equal(request.focus, focus);
      assert.match(request.meta.start_request_id, /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i);
      const message = "蓝图调整服务暂时不可用（503）";
      await browser.execute((errorMessage) => {
        const audit = window.__uiAudit;
        const index = audit.pending.findIndex((item) => item.command === "start_blueprint_refinement");
        const [request] = audit.pending.splice(index, 1);
        audit.deferred = audit.deferred.filter((command) => command !== "start_blueprint_refinement");
        request.resolve(Response.json({ code: "unavailable", message: errorMessage, retryable: true, trace_id: null, details: {} }, { status: 503 }));
      }, message);
      const progress = await dialog("调整蓝图");
      await waitText(progress, message);
      await waitText(progress, "调整内容已保留");
      const failureNotice = await browser.execute(() => ({
        details: document.querySelector('[role="dialog"] [role="alert"]')?.textContent?.trim() ?? "",
        duplicateToasts: document.querySelectorAll(".a4n-toast-container [role='alert']").length,
      }));
      assert.ok(failureNotice.details.includes("调整内容已保留"));
      assert.equal(failureNotice.duplicateToasts, 0, "The progress dialog must be the single refinement failure notice");
      assert.deepEqual(await snapshot("I09-refine-failure", width, "error-response", progress, { command: "start_blueprint_refinement", httpStatus: 503, focus }), []);
      await clickText("关闭", progress);
      await browser.waitUntil(async () => !await progress.isDisplayed(), { timeout });
      const editableForm = await dialog("调整蓝图");
      assert.equal(await editableForm.$("textarea").getValue(), focus, "The failed submission must keep the refinement input editable");
      return editableForm;
    }, check: async (scope) => ({ inputAfterFailure: await scope.$("textarea").getValue(), failedCommand: "start_blueprint_refinement", httpStatus: 503 }),
    last: async (scope) => textButton("提交优化", scope) },
    { id: "UX-1755", task: "成功提交蓝图调整后不再提示丢弃已提交的指令", open: async () => {
      const form = await blueprintRefinement();
      const focus = await form.$("textarea").getValue();
      await browser.execute(() => window.__uiAudit.deferred.push("start_blueprint_refinement"));
      await clickText("提交优化", form);
      await browser.waitUntil(async () => browser.execute(() =>
        window.__uiAudit.pending.some((item) => item.command === "start_blueprint_refinement"),
      ), { timeout });
      const request = await browser.execute(() => window.__uiAudit.pending.find((item) =>
        item.command === "start_blueprint_refinement",
      ).body);
      assert.equal(request.focus, focus);
      await respondToPending("start_blueprint_refinement", runtimeSeed.generation.blueprint.handle);

      const progress = await dialog("调整蓝图");
      await waitText(progress, "蓝图已优化");
      await clickText("查看蓝图", progress);
      await browser.waitUntil(async () => !await progress.isDisplayed(), { timeout });
      await press(await ariaButton("返回上一页"));
      await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/", { timeout });
      return await $("h1");
    }, text: ["项目工作台"],
    check: async () => ({ acceptedFocus: runtimeSeed.generation.blueprint.handle.task_id, discardedDraftPrompt: false }),
    last: async () => $("h1") },
    { id: "UX-1756", task: "明确放弃蓝图调整草稿后直接返回项目列表", open: async () => {
      const scope = await blueprintConfirmation();
      await clickText("调整蓝图", scope);
      const form = await dialog("调整蓝图");
      await form.$("textarea").setValue("尝试另一种结尾，但暂不提交。");
      await browser.keys("Escape");
      await browser.waitUntil(
        () => browser.execute(() => document.body.innerText.includes("未保存修改")),
        { timeout, timeoutMsg: "The blueprint draft discard confirmation must appear" },
      );
      await clickText("放弃草稿");
      await browser.waitUntil(async () => !await form.isDisplayed(), { timeout });
      await press(await ariaButton("返回上一页"));
      await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/", { timeout });
      return await $("h1");
    }, text: ["项目工作台"],
    check: async () => ({ confirmedDiscard: true, promptedAgainOnNavigation: false }),
    last: async () => $("h1") },
    { id: "W08-optimizer", task: "在第一章打开有历史消息的正文优化 Agent", open: async () => {
      const panel = await writingPanel("写作助手");
      await clickText("优化", panel);
      const chapterContext = await browser.execute(() => {
        const panel = document.querySelector('aside[aria-label="写作助手"]');
        const outerHeader = panel?.firstElementChild?.textContent ?? "";
        const optimizeHeader = panel?.querySelector("section > header")?.textContent ?? "";
        return { outerHeader, optimizeHeader };
      });
      assert.equal(chapterContext.outerHeader.includes("第 1 章"), false, "The optimizer panel must not repeat the chapter number in its shared mode header");
      assert.equal(chapterContext.optimizeHeader.split("第 1 章").length - 1, 1, "The optimizer header carries the single full chapter label");
      assert.ok(chapterContext.optimizeHeader.includes("雨夜来信"), "The full chapter label remains in the optimizer header");
      assert.equal(chapterContext.optimizeHeader.includes("已绑定"), false, "The bound chapter is already stated by its full label");
      const scope = await panel.$('[aria-label="领域 Agent 控制台"]');
      await waitText(scope, "当前停在讨论阶段");
      return scope;
    }, text: ["正文优化 Agent"], check: async (scope, width) => {
      const rows = await scope.$$(".agent-message-list > *");
      assert.equal(rows.length, runtimeSeed.agent.snapshot.messages.length);
      const context = await browser.execute(() => {
        const assistant = document.querySelector('aside[aria-label="写作助手"]');
        const visible = (element) => Boolean(element && getComputedStyle(element).display !== "none" && element.getClientRects().length > 0);
        const optimizerHeader = assistant?.querySelector("header");
        const optimizerTitle = optimizerHeader?.querySelector("p");
        const modalTitle = assistant?.closest('[role="dialog"]')?.querySelector("h3");
        const modalSubtitle = modalTitle?.parentElement?.querySelector(":scope > div");
        return {
          dialogs: [...document.querySelectorAll('[role="dialog"]')].map((dialog) => {
            const title = dialog.querySelector("h3");
            const subtitle = title?.parentElement?.querySelector(":scope > div");
            return {
              visible: visible(dialog),
              title: title?.textContent.trim() ?? "",
              subtitle: subtitle?.textContent.trim() ?? "",
              subtitleVisible: visible(subtitle),
            };
          }).filter((dialog) => dialog.visible),
          assistantChapterVisible: visible(assistant?.querySelector(":scope > div > span")),
          optimizerTitle: optimizerTitle?.textContent.trim() ?? "",
          repeatedBindingVisible: visible(optimizerTitle?.nextElementSibling),
          boundResource: assistant?.querySelector(".agent-console-subtitle")?.textContent.trim() ?? "",
          boundResourceVisible: visible(assistant?.querySelector(".agent-console-subtitle")),
          modalChapter: modalSubtitle?.textContent.trim() ?? "",
          modalChapterVisible: visible(modalSubtitle),
        };
      });
      current.evidence = { ...current.evidence, chapterContext: context };
      if (width >= 768) {
        assert.equal(context.assistantChapterVisible, false, "The outer assistant rail must not repeat the active chapter number");
        assert(context.optimizerTitle.includes("雨夜来信"), "The optimizer header must retain the full chapter context");
        assert.equal(context.repeatedBindingVisible, false, "The optimizer header must not repeat its chapter binding in a subtitle");
        assert.equal(context.boundResourceVisible, true, "The domain agent must retain its bound-resource status");
        assert(context.boundResource.includes("雨夜来信"), "The bound-resource status must keep the active chapter");
      } else {
        assert.equal(context.modalChapterVisible, true, "The mobile assistant dialog must expose its chapter context");
        assert(context.modalChapter.includes("第 1 章 · 雨夜来信"), "The mobile assistant dialog must name the bound chapter");
      }
      const commands = await browser.execute(() => [...window.__uiAudit.commands]);
      assert.ok(commands.includes(runtimeSeed.agent.snapshot.thread.id), "Must load the chapter-bound optimizer snapshot");
      const input = await scope.$("textarea");
      const composer = await reach(input);
      assert.ok(composer.reached, "Optimizer composer must remain user-reachable");
      await input.setValue("先给开场段的局部建议，保留原有视角与道具。");
      const send = await ariaButton("发送给领域 Agent", scope);
      await send.waitForDisplayed({ timeout });
      assert.ok(await send.isEnabled(), "The populated optimizer message must be sendable");
      const draft = await input.getValue();
      await browser.execute((sourceThreadId) => {
        const fixtures = window.__uiAudit.fixtures;
        const source = fixtures.agentSnapshots.find((item) => item.thread.id === sourceThreadId);
        const next = structuredClone(source);
        next.thread.id = "optimizer-chapter-2-fixture";
        next.thread.bound_resource = { ...next.thread.bound_resource, id: "2", label: "纸上的地图" };
        next.thread.title = "纸上的地图 · 正文优化";
        next.thread.scope = { ...next.thread.scope, resource_id: "2", chapter_id: 2, chapter_number: 2 };
        next.messages = [];
        next.events = [];
        next.executions = [];
        next.approvals = [];
        next.inbox = [];
        next.memories = [];
        next.runs = [];
        next.turns = [];
        fixtures.agentSnapshots.push(next);
      }, runtimeSeed.agent.snapshot.thread.id);
      const editor = await $('textarea[aria-label="章节正文"]');
      const panelToggle = async () => press(await $('.a4n-writing-panel-toggle button[title^="写作助手"]'));
      let activeScope = scope;
      if (width < 768) await closeDomainAgentDialog();
      await press(await ariaButton("下一章"));
      await browser.waitUntil(async () => (await editor.getValue()) === seed.commands.list_chapters[1].content, { timeout });
      if (width < 768) {
        await panelToggle();
        const panel = await $('aside[aria-label="写作助手"]');
        await clickText("优化", panel);
      }
      activeScope = await (await $('aside[aria-label="写作助手"]')).$('[aria-label="领域 Agent 控制台"]');
      await waitForOptimizerChapter("纸上的地图");
      await browser.waitUntil(async () => (await activeScope.$("textarea").getValue()) === "", { timeout });
      if (width < 768) await closeDomainAgentDialog();
      await press(await ariaButton("上一章"));
      await browser.waitUntil(async () => (await editor.getValue()) === seed.commands.list_chapters[0].content, { timeout });
      if (width < 768) {
        await panelToggle();
        const panel = await $('aside[aria-label="写作助手"]');
        await clickText("优化", panel);
      }
      activeScope = await (await $('aside[aria-label="写作助手"]')).$('[aria-label="领域 Agent 控制台"]');
      await waitForOptimizerChapter("雨夜来信");
      await browser.waitUntil(async () => (await activeScope.$("textarea").getValue()) === draft, { timeout });
      assert.equal(await activeScope.$("textarea").getValue(), draft, "Switching the bound chapter must preserve its composer draft");
      if (width < 768) await closeDomainAgentDialog();
      const beforeLeave = await browser.getUrl();
      await press(await ariaButton("返回上一页"));
      const leaveGuard = await dialog("未保存修改");
      await waitText(leaveGuard, "离开此页");
      await clickText("取消", leaveGuard);
      await browser.waitUntil(async () => await browser.getUrl() === beforeLeave, { timeout });
      if (width < 768) {
        await panelToggle();
        const panel = await $('aside[aria-label="写作助手"]');
        await clickText("优化", panel);
        activeScope = await panel.$('[aria-label="领域 Agent 控制台"]');
      }
      await browser.waitUntil(async () => (await activeScope.$("textarea").getValue()) === draft, { timeout });
      await activeScope.$("textarea").clearValue();
      return { scope: activeScope, boundThread: runtimeSeed.agent.snapshot.thread.id, messages: rows.length, composer, chapterContext: context, sendEnabled: true, chapterSwitch: "draft preserved", routeLeave: "blocked and cancelled", runCoverage: "idle only" };
    }, last: async (scope) => (await scope.$$(".agent-message-list > *")).at(-1) },
  ];
  for (const spec of pages) {
    if (spec.id === "W08-optimizer") {
      it(spec.id + " follows the runtime-backed UI path", async () => matrix(spec, [1440, 1024, 900, 390]));
    } else it(spec.id + " follows the runtime-backed UI path", async () => matrix(spec));
  }

  it("UX-2139 and UX-2144 release a bound Agent send and retain it across chapter switches", async () => {
    current = { id: "UX-2139", task: "Release the accepted bound Agent send and keep its pending bubble with the original thread", visited: [], plannedWidths: [1440] };
    await mkdir(output, { recursive: true });
    const fixtures = await buildPageFixtures(browser.options.baseUrl);
    await installUiFixtures(browser, fixtures);
    const models = await seedLocalModels(browser);
    await browser.execute(async (imageId) => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      await local.setDefaultLocalImageConfig(imageId);
    }, models.image);
    await browser.setWindowSize(1440, height);
    const panel = await writingPanel("写作助手");
    await clickText("优化", panel);
    const threadId = runtimeSeed.agent.snapshot.thread.id;
    const scope = await panel.$('[aria-label="领域 Agent 控制台"]');
    await waitText($("body"), "雨夜来信");
    await waitText($(".agent-message-list"), runtimeSeed.agent.snapshot.messages[0].content);
    const composer = await scope.$("textarea");
    const message = "先记录一条待确认的局部修改。";
    await composer.setValue(message);
    await browser.execute((id) => window.__uiAudit.deferred.push(id), threadId);
    await press(await ariaButton("发送给领域 Agent", scope));
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.agentMessages.length > 0), { timeout });
    const accepted = await browser.execute(() => {
      const request = window.__uiAudit.agentMessages.shift();
      const response = {
        thread_id: request.input.thread_id,
        message_id: `accepted-${request.input.client_message_id}`,
        turn_id: `turn-${request.input.client_message_id}`,
        run_id: `run-${request.input.client_message_id}`,
        status: "queued",
        created_run: true,
      };
      request.resolve(Response.json(response));
      return response;
    });
    await browser.waitUntil(async () => browser.execute((id) =>
      window.__uiAudit.pending.some((item) => item.command === id), threadId), { timeout });
    current.evidence = await scope.execute((root) => ({
      pendingMessageLists: root.querySelectorAll(".agent-message-list").length,
      loading: root.innerText.includes("正在加载领域 Agent"),
      threadStatus: root.querySelector(".agent-console-thread-status")?.textContent ?? null,
      messageListText: [...root.querySelectorAll(".agent-message-list")].map((list) => list.innerText),
    }));
    await browser.waitUntil(async () => browser.execute((expected) =>
      [...document.querySelectorAll(".agent-message-list")]
        .some((list) => list.innerText.includes(expected)), message), { timeout });
    await composer.setValue("下一条草稿不发送。 ");
    await browser.waitUntil(async () =>
      ariaButton("发送给领域 Agent", scope).then((send) => send.isEnabled()),
    { timeout, timeoutMsg: "The accepted Agent message must release the bound composer before its snapshot read" });
    await composer.clearValue();

    await browser.execute((sourceThreadId) => {
      const source = window.__uiAudit.fixtures.agentSnapshots.find((item) => item.thread.id === sourceThreadId);
      const next = structuredClone(source);
      next.thread.id = "optimizer-chapter-2-fixture";
      next.thread.bound_resource = { ...next.thread.bound_resource, id: "2", label: "纸上的地图" };
      next.thread.title = "纸上的地图 · 正文优化";
      next.thread.scope = { ...next.thread.scope, resource_id: "2", chapter_id: 2, chapter_number: 2 };
      next.messages = [];
      next.events = [];
      next.executions = [];
      next.approvals = [];
      next.inbox = [];
      next.memories = [];
      next.runs = [];
      next.turns = [];
      window.__uiAudit.fixtures.agentSnapshots.push(next);
    }, threadId);
    const editor = await $('textarea[aria-label="章节正文"]');
    await press(await ariaButton("下一章"));
    await browser.waitUntil(async () => (await editor.getValue()) === seed.commands.list_chapters[1].content, { timeout });
    await waitText($("body"), "纸上的地图");
    await press(await ariaButton("上一章"));
    await browser.waitUntil(async () => (await editor.getValue()) === seed.commands.list_chapters[0].content, { timeout });
    await waitText($("body"), "雨夜来信");
    await browser.waitUntil(async () => browser.execute((id) =>
      window.__uiAudit.pending.filter((item) => item.command === id).length >= 2, threadId), { timeout });
    await waitForAgentMessage(message, 15000);

    await browser.execute((id) => {
      const audit = window.__uiAudit;
      const snapshot = audit.fixtures.agentSnapshots.find((item) => item.thread.id === id);
      for (const pending of [...audit.pending]) {
        if (pending.command !== id) continue;
        pending.resolve(Response.json(snapshot));
        audit.pending = audit.pending.filter((item) => item !== pending);
      }
      audit.deferred = audit.deferred.filter((command) => command !== id);
    }, threadId);
    assert.equal(accepted.thread_id, threadId);
    assert.equal(accepted.message_id.startsWith("accepted-"), true);
  });

  it("UX-2140 and UX-2141 release bound run and approval controls after their write results", async () => {
    current = { id: "UX-2140", task: "Release bound Agent run and approval controls while snapshot refreshes are pending", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, height);
    const panel = await writingPanel("写作助手");
    const threadId = runtimeSeed.agent.snapshot.thread.id;
    await browser.execute((id, source) => {
      const snapshots = window.__uiAudit.fixtures.agentSnapshots;
      const target = snapshots.find((item) => item.thread.id === id);
      const run = structuredClone(source.runs[0]);
      run.id = "domain-agent-run";
      run.thread_id = id;
      target.runs = [run];
      target.approvals = [
        {
          ...structuredClone(source.approvals[0]),
          id: "domain-approval-first",
          execution_id: "domain-execution-first",
        },
        {
          ...structuredClone(source.approvals[0]),
          id: "domain-approval-second",
          execution_id: "domain-execution-second",
          reason: "验证前一项审批完成后下一项可操作。",
        },
      ];
      target.events = [];
      target.executions = [];
    }, threadId, homeAgentFixtures.agentSnapshots[0]);
    await clickText("优化", panel);
    const scope = await panel.$('[aria-label="领域 Agent 控制台"]');
    await waitForOptimizerChapter("雨夜来信");
    await waitText($(".agent-message-list"), runtimeSeed.agent.snapshot.messages[0].content);
    await waitText(scope, "验证前一项审批完成后下一项可操作。");

    await browser.execute((id) => window.__uiAudit.deferred.push(id), threadId);
    await press(await ariaButton("暂停", scope));
    await browser.waitUntil(async () => browser.execute((id) =>
      window.__uiAudit.pending.some((item) => item.command === id), threadId), { timeout });
    const resume = await ariaButton("继续", scope);
    await browser.waitUntil(async () => resume.isEnabled(), { timeout,
      timeoutMsg: "The bound pause result must release control before its snapshot read" });
    await browser.execute((id) => {
      const audit = window.__uiAudit;
      const pending = audit.pending.find((item) => item.command === id);
      const snapshot = audit.fixtures.agentSnapshots.find((item) => item.thread.id === id);
      pending.resolve(Response.json(snapshot));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== id);
    }, threadId);

    await browser.execute((id) => window.__uiAudit.deferred.push(id), threadId);
    const firstApproval = await scope.$('[data-approval-id="domain-approval-first"]');
    await press(await textButton("批准", firstApproval));
    await browser.waitUntil(async () => browser.execute((id) =>
      window.__uiAudit.pending.some((item) => item.command === id), threadId), { timeout });
    const nextApproval = await scope.$('[data-approval-id="domain-approval-second"]');
    const nextApprove = await textButton("批准", nextApproval);
    await browser.waitUntil(async () => nextApprove.isEnabled(), { timeout,
      timeoutMsg: "The bound approval result must release the next approval before its snapshot read" });
    await browser.execute((id) => {
      const audit = window.__uiAudit;
      const pending = audit.pending.find((item) => item.command === id);
      const snapshot = audit.fixtures.agentSnapshots.find((item) => item.thread.id === id);
      pending.resolve(Response.json(snapshot));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== id);
    }, threadId);
  });

  it("UX-2549 remounts the active detail tab when a fast reset advances its content revision", async function () {
    current = { id: "UX-2549-reset-content-revision", task: "项目重置在轮询间隔内完成时，详情 Tab 仍重读内容", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const page = await project("概览");
    await waitText(page, seed.commands.get_blueprint.one_sentence_summary);
    const blueprintReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_blueprint").length,
    );
    const projectReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project").length,
    );
    const resetCompleted = structuredClone(seed.commands.get_project);
    resetCompleted.lifecycle_gate = "active";
    resetCompleted.content_revision = 1;
    resetCompleted.status = "draft";
    resetCompleted.current_chapter = 0;
    resetCompleted.total_chapters = null;
    resetCompleted.updated_at = "2026-10-15T01:00:00Z";
    await browser.execute((project) => {
      const record = window.__uiAudit.fixtures.projectRecords.find((item) => item.id === project.id);
      Object.assign(record, project);
      window.dispatchEvent(new Event("focus"));
    }, resetCompleted);
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project").length > count,
    projectReads), { timeout });
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_blueprint").length > count,
    blueprintReads), { timeout,
      timeoutMsg: "A content revision change must reload the current detail tab even when the gate remained active" });
    assert.equal(await browser.execute(() => window.__uiAudit.unexpected.length), 0);
    assert.deepEqual(await snapshot(current.id, 1440, "content-revision-reloaded", page, {
      lifecycleGate: "active",
      contentRevision: resetCompleted.content_revision,
      projectRead: "updated",
      blueprintRead: "reloaded",
    }), []);
    current.visited.push(1440);
  });

  it("UX-2550 refreshes the project gate after a rejected material sync", async function () {
    current = { id: "UX-2550-rag-conflict-project-refresh", task: "同步资料遇到 reset 冲突后重读项目 gate", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const projectReadCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project").length,
    );
    const confirmation = await openProjectAction("同步资料索引");
    await browser.execute(() => window.__uiAudit.deferred.push("rag_ingest_all"));
    await clickText("开始同步", confirmation);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "rag_ingest_all"),
    ), { timeout });
    const resettingProject = structuredClone(seed.commands.get_project);
    resettingProject.lifecycle_gate = "resetting";
    resettingProject.content_revision = 0;
    await browser.execute((project) => {
      const audit = window.__uiAudit;
      const record = audit.fixtures.projectRecords.find((item) => item.id === project.id);
      Object.assign(record, project);
      audit.deferred.push("get_project");
      const pending = audit.pending.find((item) => item.command === "rag_ingest_all");
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "rag_ingest_all");
      pending.resolve(Response.json({
        code: "project_lifecycle_blocked",
        message: "项目正在重置生成数据，不能同步资料",
        retryable: false,
        trace_id: null,
        details: {},
      }, { status: 409 }));
    }, resettingProject);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "get_project"),
    ), { timeout, timeoutMsg: "A rejected RAG write must refresh the project status" });
    await respondToPending("get_project", resettingProject);
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project").length > count,
    projectReadCount), { timeout });
    const progress = await dialog("同步资料索引");
    await waitText(progress, "项目正在重置生成数据");
    await clickText("关闭", progress);
    await press(await $("button[title=\"更多项目操作\"]"));
    const actions = await dialog();
    assert.equal(await containing("同步资料索引", actions).isExisting(), false);
    assert.equal(await browser.execute(() => window.__uiAudit.unexpected.length), 0);
    assert.deepEqual(await snapshot(current.id, 1440, "project-gate-refreshed", actions, {
      rejectedCommand: "rag_ingest_all",
      refreshedGate: "resetting",
    }), []);
    current.visited.push(1440);
  });

  it("UX-2551 refreshes the project gate after blueprint refinement is rejected", async function () {
    current = { id: "UX-2551-refine-conflict-project-refresh", task: "蓝图优化遇到 reset 冲突后重读项目 gate", visited: [], plannedWidths: [1440] };
    current.width = 1440;
    await browser.setWindowSize(1440, height);
    const projectReadCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project").length,
    );
    const form = await openProjectAction("优化蓝图");
    await form.$("textarea").setValue("跨窗口重置期间不得继续提交旧蓝图。");
    await browser.execute(() => window.__uiAudit.deferred.push("start_blueprint_refinement"));
    await clickText("开始优化", form);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "start_blueprint_refinement"),
    ), { timeout });
    const resettingProject = structuredClone(seed.commands.get_project);
    resettingProject.lifecycle_gate = "resetting";
    resettingProject.content_revision = 0;
    await browser.execute((project) => {
      const audit = window.__uiAudit;
      const record = audit.fixtures.projectRecords.find((item) => item.id === project.id);
      Object.assign(record, project);
      audit.deferred.push("get_project");
      const pending = audit.pending.find((item) => item.command === "start_blueprint_refinement");
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "start_blueprint_refinement");
      pending.resolve(Response.json({
        code: "project_lifecycle_blocked",
        message: "项目正在重置生成数据，不能优化蓝图",
        retryable: false,
        trace_id: null,
        details: {},
      }, { status: 409 }));
    }, resettingProject);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "get_project"),
    ), { timeout, timeoutMsg: "A rejected blueprint refinement must refresh the project status" });
    await respondToPending("get_project", resettingProject);
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_project").length > count,
    projectReadCount), { timeout });
    await browser.waitUntil(async () => {
      const toast = $(".a4n-toast");
      return await toast.isExisting() && (await toast.getText()).includes("项目正在重置生成数据");
    }, { timeout });
    await press(await $("button[title=\"更多项目操作\"]"));
    const actions = await dialog();
    assert.equal(await containing("优化蓝图", actions).isExisting(), false);
    assert.equal(await browser.execute(() => window.__uiAudit.unexpected.length), 0);
    assert.deepEqual(await snapshot(current.id, 1440, "project-gate-refreshed", actions, {
      rejectedCommand: "start_blueprint_refinement",
      refreshedGate: "resetting",
    }), []);
    current.visited.push(1440);
  });
});
