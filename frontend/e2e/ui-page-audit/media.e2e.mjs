import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { buildPageFixtures, seedLocalModels } from "./fixtures.mjs";

const output = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/media/", import.meta.url));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";
const widths = [1440, 900, 390];
const canvasWidths = [1440, 1024, 390];
const report = [];
let fixtures;
const dialog = '.a4n-modal-layer:not([inert])[data-modal-state="entering"] > [role="dialog"]';
const comicPage = 'nav[aria-label="漫画页列表"] + section > article';
const comicPrompt = `${comicPage} > div:last-child > div > section`;
const galgameEditor = 'section[aria-label="编辑"]';
const galgameLines = `${galgameEditor} article`;
const timeline = '[aria-label="视频剪辑时间轴"]';
const resultPane = 'section[aria-label="视频结果"]';

const buttonText = text => $(`//button[normalize-space()=${JSON.stringify(text)}]`);
async function visibleButtonText(text) {
  for (const button of await $$(`//button[normalize-space()=${JSON.stringify(text)}]`)) {
    if (await button.isDisplayed()) return button;
  }
  throw new Error(`Visible button not found: ${text}`);
}
const comicFixture = page => fixtures.byChapter[1].list_manga_page_prompts.find(row => row.page_number === page);

async function ready() {
  await browser.execute(async () => { await document.fonts.ready; });
  await browser.pause(220);
}

async function openRoute(path) {
  await browser.url("/");
  await browser.switchToWindow(await browser.getWindowHandle());
  await $(".a4n-home-content").waitForDisplayed();
  await browser.execute(() => {
    for (const key of Object.keys(localStorage)) {
      if (key.startsWith("a4n:video:") || key.startsWith("a4n:video-canvas:")) localStorage.removeItem(key);
    }
  });
  await browser.url(path);
  await browser.switchToWindow(await browser.getWindowHandle());
  await $(".a4n-page-header h1").waitForDisplayed();
  await ready();
}

async function openMenu(scope, label, index = 0) {
  const triggers = await $(scope).$$('button[aria-haspopup="menu"]');
  assert(triggers[index], `Missing menu ${index} in ${scope}`);
  await triggers[index].click();
  const item = await $(`//*[@role="menuitem" and normalize-space()=${JSON.stringify(label)}]`);
  await item.waitForClickable();
  await item.click();
  await ready();
}

async function openDetails(selector) {
  const details = await $(selector);
  await details.waitForExist();
  const summary = await details.$("summary");
  const reached = await reachByWheel(summary);
  assert.equal(reached.reached, true, `Disclosure summary must be reachable through its scroll container: ${selector}`);
  if (!(await details.getProperty("open"))) await summary.click();
  await browser.waitUntil(async () => (await details.getProperty("open")) === true, {
    timeout: 2500, timeoutMsg: `Details did not open: ${selector}`,
  });
  await ready();
}

async function selectOption(trigger, label) {
  await trigger.click();
  const option = await $(`//*[@role="option" and normalize-space()=${JSON.stringify(label)}]`);
  await option.waitForClickable();
  await option.click();
  await ready();
}

async function openComic(page = 1, view = "prompt", chapter = 1) {
  await openRoute("/comic-project/2");
  const stageId = view === "references" ? "visual_design" : "image_creation";
  const stageLabel = view === "references" ? "分镜与参考图" : "提示词与成图";
  const select = await $('[role="combobox"][id="comic-stages-2-select"]');
  if (await select.isDisplayed()) {
    await select.click();
    const option = await $(`//*[@role="option" and contains(normalize-space(.), ${JSON.stringify(stageLabel)})]`);
    await option.waitForClickable();
    await option.click();
  } else {
    const stage = await $(`button[id="comic-stages-2-tab-${stageId}"]`);
    await stage.waitForDisplayed();
    if (await stage.getAttribute("aria-selected") !== "true") await stage.click();
  }
  await ready();
  const chapterNav = $('nav[aria-label="漫画章节"]');
  await chapterNav.waitForDisplayed({ timeout: 15_000 });
  const chapterButtons = await chapterNav.$$("ul > li > button");
  assert.equal(chapterButtons.length, 3, "The comic audit needs all three source chapters");
  await chapterButtons[chapter - 1].click();
  await $('nav[aria-label="漫画页列表"]').waitForDisplayed();
  await selectComicPage(page);
  await ready();
}

async function selectComicPage(page) {
  const selector = `//nav[@aria-label="漫画页列表"]//button[.//span[normalize-space()="第 ${page} 页"]]`;
  const pageButton = await $(selector);
  await reachByWheel(selector, "y");
  const axis = await $('nav[aria-label="漫画页列表"]').execute(element => element.scrollWidth > element.clientWidth + 1 ? "x" : "y");
  const reached = await reachByWheel(selector, axis);
  assert.equal(reached.reached, true, `Comic page ${page} must be reachable through its scroll rail`);
  await pageButton.click();
  await browser.waitUntil(async () => (await pageButton.getAttribute("aria-current")) === "true");
  await ready();
}

async function openVideo(tab = "片段生成") {
  await openRoute("/video-project/3");
  const mobileSelector = await $('[role="combobox"][id="video-stages-3-select"]');
  if (await mobileSelector.isDisplayed()) {
    await mobileSelector.waitForDisplayed();
    await mobileSelector.click();
    const option = await $(`//*[@role="option" and contains(normalize-space(.), ${JSON.stringify(tab)})]`);
    await option.waitForClickable();
    await option.click();
  } else {
    const target = await $(`//*[@role="tab" and contains(normalize-space(.), ${JSON.stringify(tab)})]`);
    await target.waitForClickable();
    await target.click();
  }
  await ready();
}

async function openVideoMaterials() {
  await openVideo("基础素材");
  await buttonText("场景与道具").click();
  await $("article[data-state]").waitForDisplayed();
  await ready();
}

async function selectSegment(number) {
  const target = await $(`//button[@data-video-segment="${number}" and not(ancestor::*[@role="dialog"])]`);
  await target.waitForClickable();
  if ((await target.getAttribute("aria-current")) !== "true") await target.click();
  await browser.waitUntil(async () => (await target.getAttribute("aria-current")) === "true");
  return target;
}

async function openSegment(number) {
  await openVideo();
  const target = await selectSegment(number);
  await target.click();
  await $(dialog).waitForDisplayed();
  await $(`${dialog} [aria-label="片段 ${number} 生成方案"]`).waitForDisplayed();
  await ready();
}

async function openFrame(number, label) {
  await openSegment(number);
  const slot = await $(`${dialog} button[aria-label="配置${label}"]`);
  // The label is an actual configuration target; image-click behavior has its own regression case.
  await slot.$(`.//span[normalize-space()=${JSON.stringify(label)}]`).click();
  await $(dialog).waitForDisplayed();
  await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === `${label}配置`);
  await ready();
}

async function openVideoResult() {
  await openVideo();
  await selectSegment(1);
  if (await $('[aria-label="视频生成移动工作区"]').isDisplayed()) {
    const switches = await $('[aria-label="视频生成移动工作区"]').$$("button");
    await switches[1].click();
  }
  await $(`${resultPane} [data-video-preview-id="31001"]`).waitForDisplayed();
  await ready();
}

async function openVideoPrompt(edit = false) {
  await openVideoResult();
  await $(`${resultPane} button[title="查看完整视频提示词"]`).click();
  await $(dialog).waitForDisplayed();
  if (edit) await $(dialog).$('.//button[normalize-space()="编辑"]').click();
  await ready();
}

async function openMaterialDetails(tab = "概览") {
  await openVideoMaterials();
  const target = await $('article[data-state="ready"] button[aria-label="打开素材图"]');
  await target.waitForClickable();
  await target.click();
  await $(dialog).waitForDisplayed();
  await $(`${dialog} [aria-label="图片详情栏目"]`).$(`.//button[normalize-space()=${JSON.stringify(tab)}]`).click();
  await ready();
}

async function openGalgame(view = "editor", { openSceneProjection = true } = {}) {
  await openRoute("/galgame-project/5");
  const select = await $('[role="combobox"][id="galgame-stages-5-select"]');
  if (await select.isDisplayed()) {
    await select.click();
    const option = await $('//*[@role="option" and contains(normalize-space(.), "剧本")]');
    await option.waitForClickable();
    await option.click();
    await ready();
  } else {
    const script = await $('button[id="galgame-stages-5-tab-script"]');
    if (await script.getAttribute("aria-selected") !== "true") await script.click();
  }
  await browser.waitUntil(async () => (await $$(galgameLines)).length === 8, {
    timeoutMsg: "The first Galgame scene must contain eight real line editors",
  });
  if (openSceneProjection) {
    await openDetails('//details[./summary[normalize-space()="场景投影（只读）"]]');
    if (await $('[aria-label="GALGame 移动工作区"]').isDisplayed()) {
      await $('[aria-label="GALGame 移动工作区"]').$(`.//button[normalize-space()="${view === "editor" ? "编辑" : "场次"}"]`).click();
    }
  }
  await ready();
}

async function installStoryGraphReviewFixture(graph) {
  const project = fixtures.byProject[5];
  await installUiFixtures(browser, {
    ...fixtures,
    byProject: {
      ...fixtures.byProject,
      5: {
        ...project,
        get_galgame_script: {
          ...project.get_galgame_script,
          story_graph: graph,
          story_graph_status: "generated",
        },
      },
    },
  });
}

async function openGalgameScriptControls() {
  await openGalgame();
  await openDetails('//details[./summary[normalize-space()="场景投影（只读）"]]');
  const mobileWorkspace = await $('[aria-label="GALGame 移动工作区"]');
  if (await mobileWorkspace.isDisplayed()) {
    await mobileWorkspace.$('button=场次').click();
    await ready();
  }
}

async function openGalgameAssets() {
  await openGalgame("scenes");
  await chooseGalgameStage("asset_plan", "资产规划");
  const entry = await buttonText("规划视觉资产");
  await entry.waitForClickable();
  await entry.click();
  await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.commands.includes("plan_galgame_assets")), {
    timeout: 2500, timeoutMsg: "The asset planning click did not issue its real fixture request",
  });
  await browser.waitUntil(async () => (await $$('section[aria-label="资产规划"] tbody td')).length === 12, {
    timeout: 2500, timeoutMsg: "The asset planning dialog must render all twelve portrait cells",
  });
  await ready();
}

async function chooseGalgameStage(stageId, label) {
  if (await browser.execute(() => innerWidth) < 640) {
    const select = await $('[role="combobox"][id="galgame-stages-5-select"]');
    await select.click();
    const option = await $(`//*[@role="option" and contains(normalize-space(.), ${JSON.stringify(label)})]`);
    await option.waitForClickable();
    await option.click();
  } else {
    const stage = await $(`button[id="galgame-stages-5-tab-${stageId}"]`);
    if (await stage.getAttribute("aria-selected") !== "true") await stage.click();
  }
  await ready();
}

async function openGalgamePerformance(line) {
  await openGalgame();
  await openDetails(`${galgameLines}:nth-of-type(${line}) > details[class*="group/performance"]`);
}

async function openSourceSync(projectId) {
  const route = projectId === 2 ? "/comic-project/2" : projectId === 3 ? "/video-project/3" : "/galgame-project/5";
  await openRoute(route);
  const syncSelector = projectId === 3
    ? 'button[aria-label="同步来源"]'
    : '.a4n-page-header button[aria-label="同步来源"]';
  assert.equal((await $$(syncSelector)).length, 1, "The current project must expose one exact source synchronization entry");
  await $(syncSelector).click();
  await $(dialog).waitForDisplayed();
  await $(dialog).$('.//button[normalize-space()="检查更新"]').click();
  await browser.waitUntil(async () => (await $$(`${dialog} article`)).length === 3, {
    timeoutMsg: "Source synchronization must render all three typed conflicts",
  });
  const requests = await browser.execute(() => window.__uiAudit.requests
    .filter(item => item.command === "preview_source_sync")
    .map(item => item.body.input.derived_project_id));
  assert.deepEqual(requests, [projectId], "Source synchronization must preview the current derived project");
  const rows = await $$(`${dialog} article`);
  const plan = fixtures.byProject[projectId].preview_source_sync;
  for (let index = 0; index < rows.length; index++) {
    const columns = await rows[index].$$(':scope > div:nth-child(2) > div');
    assert.equal(columns.length, 2, "Every conflict must compare source and local summaries");
    for (const [side, key] of ["source_summary_json", "local_summary_json"].entries()) {
      const text = await columns[side].getText();
      for (const value of Object.values(plan.conflicts[index][key])) {
        if (typeof value === "string") assert(text.includes(value), `Conflict ${index + 1} ${key} omitted its stored summary`);
      }
    }
  }
  await ready();
}

async function inspectSourceSyncEnd(record, width) {
  record.conflictViews = [];
  for (let index = 1; index <= 3; index++) {
    const reached = await reachByWheel(`${dialog} article:nth-of-type(${index}) > details > summary`);
    assert.equal(reached.reached, true, `Source conflict ${index} must be reachable with real scrolling`);
    record.conflictViews.push(await capture({ id: record.id, scope: dialog }, width, index === 3 ? "-conflicts-end" : `-conflict-${index}`));
  }
  record.conflictsEnd = record.conflictViews.at(-1);
  await browser.keys("Escape");
  await $(dialog).waitForExist({ reverse: true });
  const calls = await browser.execute(() => window.__uiAudit.commands);
  assert.equal(calls.includes("apply_source_sync_decisions"), false);
  assert.equal(calls.includes("detach_derived_project"), false);
  return { conflictsRead: 3, finalConflictReachable: true, mutated: false };
}

async function inspectVideoEditorEnd(record) {
  const target = record.id === "MV15"
    ? `${dialog} textarea[aria-label^="音频 1 "]`
    : `${dialog} fieldset > div:last-child > label:last-child > textarea`;
  await $(target).waitForExist();
  const reached = await reachByWheel(target);
  assert.equal(reached.reached, true, "The final populated video field must be reachable with real scrolling");
  assert((await $(target).getValue()).trim().length > 0, "The final video field must contain its real fixture value");
  await $(target).click();
  return { finalFieldReachable: true, field: record.id === "MV15" ? "audioUsage" : "negativePrompt" };
}

async function openCanvas(nodeId = null) {
  await openRoute("/canvas-project/6");
  await browser.waitUntil(async () => (await $$(".react-flow__node")).length === 17, {
    timeoutMsg: "The media graph must contain seventeen populated nodes",
  });
  if (nodeId === null) return;
  const node = await $(`.react-flow__node[data-id="${nodeId}"]`);
  // React Flow's keyboard-focus path pans distant nodes into view without changing app state.
  await browser.keys("Tab");
  await node.execute(element => element.focus());
  await ready();
  const icon = await node.$(".a4n-studio-node-icon");
  if (!(await icon.isClickable())) {
    // A partially visible node does not trigger React Flow's focus pan; use the visible overview control.
    await $('.react-flow__controls-fitview').click();
    await browser.keys("Tab");
    await node.execute(element => element.focus());
    await ready();
  }
  await icon.click();
  await browser.waitUntil(async () => (await node.getAttribute("class")).includes("selected"));
  const width = await browser.execute(() => innerWidth);
  if (width <= 1100) {
    await openCanvasProperties();
    await $(dialog).waitForDisplayed();
  }
  await ready();
}

async function openCanvasProperties() {
  const trigger = await $('.a4n-video-canvas-header button[aria-label="更多操作"]');
  await trigger.click();
  const action = await $('//*[@role="menuitem" and normalize-space(.)="打开属性面板"]');
  await action.waitForClickable();
  await action.click();
}

async function openCanvasThumbnail() {
  await openCanvas();
  const node = await $('.react-flow__node[data-id="6006"]');
  await node.execute(element => element.focus());
  await ready();
  const image = await node.$("img");
  await browser.waitUntil(async () => image.execute(element => element.complete && element.naturalWidth > 0), {
    timeout: 2500, timeoutMsg: "The existing video node must show its decoded thumbnail",
  });
  const expected = fixtures.byProject[6].get_canvas_graph.nodes.find(item => item.id === 6006).config_json.thumbnail_path;
  assert.equal(await image.execute(element => element.currentSrc), expected);
}

async function canvasInspector() {
  return (await browser.execute(() => innerWidth)) <= 1100
    ? dialog : 'section[aria-label="视频画布编辑区"] > aside:last-child';
}

async function openCanvasDraft(label) {
  await openCanvas();
  const library = await $('button[aria-label="打开节点库"]');
  if (await library.isDisplayed()) {
    await library.click();
    await $(dialog).waitForDisplayed();
  }
  const template = await $(`//button[@draggable and normalize-space()=${JSON.stringify(label)}]`);
  assert.equal((await reachByWheel(template)).reached, true, "The chosen template must be reached through its real node library");
  await template.click();
  await browser.waitUntil(async () => (await $$(".react-flow__node")).length === 18, {
    timeout: 2500, timeoutMsg: "The library must create exactly one local draft beside the seventeen saved nodes",
  });
  const selected = await $$(".react-flow__node.selected");
  assert.equal(selected.length, 1);
  assert.match(await selected[0].getAttribute("data-id"), /^node-/);
  if ((await browser.execute(() => innerWidth)) <= 1100) {
    await $(dialog).waitForExist({ reverse: true });
    await openCanvasProperties();
    await $(dialog).waitForDisplayed();
  }
  await ready();
}

async function inspectCanvasDraft(kind) {
  const scope = await canvasInspector();
  const identity = await $(scope).$('.//details[./div/span[normalize-space()="节点类型"]]');
  assert.equal((await reachByWheel(await identity.$("summary"))).reached, true);
  await identity.$("summary").click();
  assert.equal(await identity.$('.//span[normalize-space()="节点类型"]/following-sibling::span[1]').getText(), kind);
  assert.equal(await $(scope).$('.//button[normalize-space()="查看内容"]').isExisting(), false, "An unrun draft must not claim to contain an output");
  assert.equal(await browser.execute(() => window.__uiAudit.commands.some(command => ["save_canvas_graph", "run_canvas_node", "run_canvas_graph"].includes(command))), false);
  return { kind, nodes: 18, saved: false, executed: false, hasSavedContent: false };
}

async function openCanvasContent(nodeId, label) {
  await openCanvas(nodeId);
  const scope = await canvasInspector();
  await $(scope).$('.//button[normalize-space()="查看内容"]').click();
  await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === `${label} 内容`);
  await ready();
}

async function openCanvasEdge() {
  await openCanvas();
  const edge = await $('.react-flow__edge[data-id="6801"]');
  const pointOnEdge = () => edge.$(".react-flow__edge-interaction").execute(path => {
    const matrix = path.getScreenCTM();
    const length = path.getTotalLength();
    if (!matrix || length <= 0) throw new Error("The selected edge has no rendered SVG geometry");
    const edge = path.closest('.react-flow__edge');
    for (const fraction of [0.5, 0.25, 0.75, 0.1, 0.9]) {
      const local = path.getPointAtLength(length * fraction);
      const screen = new DOMPoint(local.x, local.y).matrixTransform(matrix);
      const x = Math.round(screen.x), y = Math.round(screen.y);
      if (x < 0 || y < 0 || x >= innerWidth || y >= innerHeight) continue;
      const hit = document.elementFromPoint(x, y);
      if (hit && edge.contains(hit)) return { x, y };
    }
    return null;
  });
  let point = await pointOnEdge();
  if (!point) {
    await $('.react-flow__controls-fitview').click();
    await ready();
    point = await pointOnEdge();
  }
  assert(point, "The edge must expose a real unoccluded hit point after using the overview control");
  await browser.performActions([{ type: "pointer", id: "canvas-edge-pointer", parameters: { pointerType: "mouse" }, actions: [
    { type: "pointerMove", origin: "viewport", x: point.x, y: point.y },
    { type: "pointerDown", button: 0 }, { type: "pointerUp", button: 0 },
  ] }]);
  await browser.releaseActions();
  await browser.waitUntil(async () => (await edge.getAttribute("class")).includes("selected"));
  if ((await browser.execute(() => innerWidth)) <= 1100) {
    await openCanvasProperties();
    await $(dialog).waitForDisplayed();
  }
  await openDetails(`${await canvasInspector()} details`);
}

async function openPostprocess(markers = 0) {
  await openVideo("成片后处理");
  await $('[data-video-push-id="31081"]').click();
  await $(timeline).waitForDisplayed();
  await waitForPlayable(".a4n-page-container video");
  for (const fraction of [0.25, 0.75].slice(0, markers)) await clickFraction(timeline, fraction);
  if (markers > 0) {
    assert.equal((await reachByWheel(timeline)).reached, true, "The selected range must remain reachable after its action toolbar wraps");
  }
  await ready();
}

async function openInserted() {
  await openVideo();
  await $('button[data-video-segment="1"]').waitForDisplayed();
  const gaps = await $$('button[aria-label="插入片段"]');
  assert(gaps.length > 1, "A populated timeline must expose insertion gaps");
  await gaps[1].click();
  await $(dialog).waitForDisplayed();
  await ready();
}

async function clickFraction(selector, fraction) {
  const reached = await reachByWheel(selector);
  assert.equal(reached.reached, true, "The timeline must be reached by real scrolling before pointer input");
  const box = await $(selector).execute(element => {
    const { x, y, width, height } = element.getBoundingClientRect();
    return { x, y, width, height };
  });
  await browser.performActions([{ type: "pointer", id: "media-pointer", parameters: { pointerType: "mouse" }, actions: [
    { type: "pointerMove", origin: "viewport", x: Math.round(box.x + box.width * fraction), y: Math.round(box.y + box.height / 2) },
    { type: "pointerDown", button: 0 }, { type: "pointerUp", button: 0 },
  ] }]);
  await browser.releaseActions();
}

async function waitForPlayable(selector) {
  await browser.waitUntil(async () => $(selector).execute(element =>
    element.readyState >= 2 && Number.isFinite(element.duration) && element.duration > 0
      && (!element.tagName.includes("VIDEO") || element.videoWidth > 0)), {
    timeout: 10000, timeoutMsg: `No decoded media in ${selector}`,
  });
}

async function provePlayback(selector) {
  await waitForPlayable(selector);
  const before = await $(selector).execute(async element => {
    element.muted = true;
    const time = element.currentTime;
    await element.play();
    return time;
  });
  await browser.waitUntil(async () => $(selector).execute((element, start) => element.currentTime > start + 0.1, before), {
    timeout: 4000, timeoutMsg: "Decoded media did not advance while playing",
  });
  return $(selector).execute(element => { element.pause(); return { time: element.currentTime, duration: element.duration }; });
}

async function reachByWheel(selector, axis = "y") {
  const target = typeof selector === "string" ? await $(selector) : selector;
  const state = () => target.execute((element, direction) => {
    const box = element.getBoundingClientRect();
    const x = box.x + box.width / 2, y = box.y + box.height / 2;
    const hit = x >= 0 && x < innerWidth && y >= 0 && y < innerHeight ? document.elementFromPoint(x, y) : null;
    const reached = Boolean(hit && (element === hit || element.contains(hit)));
    let scroll = element.parentElement;
    while (scroll) {
      const style = getComputedStyle(scroll);
      const overflow = direction === "x" ? style.overflowX : style.overflowY;
      const extent = direction === "x" ? scroll.scrollWidth - scroll.clientWidth : scroll.scrollHeight - scroll.clientHeight;
      if (/auto|scroll/.test(overflow) && extent > 1) break;
      scroll = scroll.parentElement;
    }
    const surface = scroll?.getBoundingClientRect();
    const bounds = surface ? {
      left: Math.max(0, surface.left), right: Math.min(innerWidth, surface.right),
      top: Math.max(0, surface.top), bottom: Math.min(innerHeight, surface.bottom),
    } : { left: 0, right: innerWidth, top: 0, bottom: innerHeight };
    const center = { x: (bounds.left + bounds.right) / 2, y: (bounds.top + bounds.bottom) / 2 };
    const span = direction === "x" ? bounds.right - bounds.left : bounds.bottom - bounds.top;
    const distance = direction === "x" ? x - center.x : y - center.y;
    if (!scroll && document.scrollingElement?.scrollHeight > document.scrollingElement?.clientHeight + 1) {
      scroll = document.scrollingElement;
    }
    return {
      reached, scrollable: Boolean(scroll), target: { x, y }, bounds,
      scrollTarget: scroll?.className || scroll?.id || scroll?.tagName || null,
      x: Math.round(Math.max(2, Math.min(innerWidth - 2, center.x))),
      y: Math.round(Math.max(2, Math.min(innerHeight - 2, center.y))),
      delta: Math.round(Math.max(-span / 2, Math.min(span / 2, distance))),
    };
  }, axis);
  let value = await state();
  for (let attempt = 0; attempt < 20 && !value.reached && value.scrollable && value.delta !== 0; attempt++) {
    await browser.performActions([{ type: "wheel", id: "media-wheel", actions: [{
      type: "scroll", origin: "viewport", x: value.x, y: value.y,
      deltaX: axis === "x" ? value.delta : 0, deltaY: axis === "y" ? value.delta : 0, duration: 120,
    }] }]);
    await browser.releaseActions();
    await browser.pause(120);
    value = await state();
  }
  return value;
}

async function focusTextEnd(selector) {
  const field = await $(selector);
  const reached = await reachByWheel(field);
  assert.equal(reached.reached, true, "The text editor must be reachable with real scrolling");
  await field.click();
  await browser.keys(["Control", "End"]);
  await browser.releaseActions();
  assert.equal(await field.execute(element => element === document.activeElement && element.selectionEnd === element.value.length), true);
}

async function capture(entry, width, suffix = "") {
  const file = `${phase}-${entry.id}-${width}${suffix}.png`;
  const state = await browser.execute((selector) => {
    const scope = selector ? document.querySelector(selector) : document.body;
    if (!scope) throw new Error(`Missing audit scope: ${selector}`);
    const visible = element => {
      const box = element.getBoundingClientRect(), css = getComputedStyle(element);
      for (let parent = element.parentElement; parent; parent = parent.parentElement) {
        if (parent.tagName === "DETAILS" && !parent.open && !parent.querySelector(":scope > summary")?.contains(element)) return false;
      }
      return box.width > 0 && box.height > 0 && box.bottom > 0 && box.top < innerHeight
        && css.visibility !== "hidden" && css.display !== "none" && !element.closest("[inert]");
    };
    const label = element => element.getAttribute("aria-label")
      || [...(element.labels ?? [])].map(item => item.textContent.trim()).join(" ")
      || (element.getAttribute("aria-labelledby") ?? "").split(/\s+/).map(id => document.getElementById(id)?.textContent ?? "").join(" ").trim()
      || element.closest("label")?.textContent.trim() || "";
    const controls = [...scope.querySelectorAll("button,input:not([type=hidden]):not([type=password]),textarea,summary,[role=slider]")].filter(visible).map(element => {
      const box = element.getBoundingClientRect();
      return { tag: element.tagName, label: label(element) || element.textContent.trim().slice(0, 80), width: Math.round(box.width), height: Math.round(box.height), disabled: element.disabled ?? false, value: "value" in element ? String(element.value).slice(0, 100) : null, outside: box.left < -1 || box.right > innerWidth + 1 };
    });
    const images = [...scope.querySelectorAll('img:not([aria-hidden="true"])')].filter(visible).map(image => ({ alt: image.alt, loaded: image.complete && image.naturalWidth > 0, src: image.currentSrc }));
    return {
      url: location.pathname + location.search + location.hash, width: innerWidth, height: innerHeight,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      text: scope.innerText.slice(0, 12000), controls, images,
      unnamedFields: [...scope.querySelectorAll('input:not([type=hidden]):not([type=password]),textarea')].filter(visible).filter(element => !label(element)).map(element => ({ tag: element.tagName, placeholder: element.placeholder })),
      selectedTabs: [...document.querySelectorAll('[role=tab][aria-selected=true]')].map(element => element.textContent.trim()),
      dialogTitles: [...document.querySelectorAll('.a4n-modal-layer:not([inert])[data-modal-state="entering"] [role=dialog]')].map(element => document.getElementById(element.getAttribute("aria-labelledby"))?.textContent),
      commands: window.__uiAudit.requests.map(({ command, body }) => ({ command, projectId: body.input?.project_id ?? body.input?.derived_project_id ?? body.projectId ?? null, chapterId: body.input?.chapter_id ?? body.input?.source_chapter_id ?? body.chapterId ?? null })),
      unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors,
    };
  }, entry.scope ?? null);
  await writeFile(`${output}/${file}`, Buffer.from(await browser.takeScreenshot(), "base64"));
  return { file, ...state };
}

const cases = [
  { id: "MC01", title: "漫画来源章节", task: "辨认三个来源章节及其生成状态", open: () => openRoute("/comic-project/2"), expected: ["雨夜来信", "纸上的地图", "黎明的站台"], count: ['nav[aria-label="漫画章节"] ul > li > button', 3], note: "窄屏章节轨道隐藏滚动条，检查末章发现性。" },
  {
    id: "MC15",
    title: "漫画素材切换刷新",
    task: "离开基础素材后刷新角色立绘和项目图片候选",
    open: async () => {
      await browser.url("/");
      await browser.execute(() => localStorage.setItem("a4n:comic:2:stage", "materials"));
      await openRoute("/comic-project/2");
      await $('nav[aria-label="漫画章节"]').waitForDisplayed();
      const readCounts = () => browser.execute(() => {
        const commands = ["list_profiles", "list_portraits_by_project", "list_images_by_projects"];
        return Object.fromEntries(commands.map(command => [
          command,
          window.__uiAudit.requests.filter(request => request.command === command).length,
        ]));
      });
      await browser.waitUntil(async () =>
        Object.values(await readCounts()).every(count => count > 0),
      );
      const before = await readCounts();
      const chapterButtons = await $('nav[aria-label="漫画章节"]').$$("ul > li > button");
      await chapterButtons[0].click();
      await ready();
      if (await browser.execute(() => innerWidth) < 640) {
        const stageSelect = await $('[role="combobox"][id="comic-stages-2-select"]');
        await stageSelect.click();
        const option = await $(`//*[@role="option" and contains(normalize-space(.), ${JSON.stringify("页面提示词")})]`);
        await option.waitForClickable();
        await option.click();
      } else {
        await $('button[id="comic-stages-2-tab-prompts"]').click();
      }
      await $('nav[aria-label="漫画页列表"]').waitForDisplayed();
      await browser.waitUntil(async () => {
        const after = await readCounts();
        return Object.keys(before).every(command => after[command] > before[command]);
      });
    },
    expected: ["参考图绑定", "林岚"],
    count: ['nav[aria-label="漫画页列表"] button', 6],
    note: "离开素材阶段后，角色档案、立绘和项目图片候选都应读取最新数据。",
  },
  { id: "MC02", title: "漫画分镜与参考", task: "查看当前页的分镜布局与角色参考发现", open: () => openComic(1, "references"), expected: ["发现参考图", "林岚", "周衡"], count: ['nav[aria-label="漫画页列表"] button', 6], note: "检查章节状态、页状态与操作栏是否挤压正文。" },
  { id: "MC03", title: "漫画提示词编辑", task: "修改提示词并确认保存入口及取消恢复", open: async () => { await openComic(); await (await visibleButtonText("编辑提示词")).click(); await $(`${comicPrompt} textarea`).addValue("\n审查草稿"); }, expected: ["保存", "取消"], after: async () => { await $(comicPrompt).$('.//button[normalize-space()="取消"]').click(); assert.equal(await $(`${comicPrompt} textarea`).isExisting(), false); return { cancelled: true }; }, note: "通过当前页的直接编辑按钮修改；保留草稿取消，不触发下游写回。" },
  { id: "MC04", title: "漫画缺失角色参考", task: "查看第4页缺失的角色参考及候选", open: () => openComic(4, "references"), expected: ["发现参考图", "周衡"], tail: '(//nav[@aria-label="漫画页列表"]/following-sibling::section/article//button[@role="combobox"])[last()]', note: "候选选择后的图像是否仍显示旧绑定，需要视觉复核。" },
  { id: "MC05", title: "漫画成图", task: "阅读完整漫画页并查看尺寸", open: () => openComic(1, "image"), scope: comicPage, expected: ["漫画成图"], after: async (_record, width) => {
    const initial = await browser.execute((selector) => {
      const image = document.querySelector(selector);
      if (!image) return null;
      const bounds = image.getBoundingClientRect();
      const rect = (element) => {
        if (!element) return null;
        const box = element.getBoundingClientRect();
        return { top: Math.round(box.top), bottom: Math.round(box.bottom), height: Math.round(box.height) };
      };
      const visibleButton = (label) => [...document.querySelectorAll("button")].find((button) => {
        const style = getComputedStyle(button);
        return button.textContent.trim() === label && style.display !== "none" && style.visibility !== "hidden" && button.getClientRects().length > 0;
      });
      const scrollArea = image.closest(".overflow-y-auto");
      const scrollBounds = scrollArea?.getBoundingClientRect();
      const visibleTop = Math.max(bounds.top, scrollBounds?.top ?? 0, 0);
      const visibleBottom = Math.min(bounds.bottom, scrollBounds?.bottom ?? innerHeight, innerHeight);
      return {
        top: Math.round(bounds.top),
        bottom: Math.round(bounds.bottom),
        height: Math.round(bounds.height),
        viewportHeight: innerHeight,
        visibleHeight: Math.max(0, Math.round(visibleBottom - visibleTop)),
        pageActions: {
          edit: rect(visibleButton("编辑提示词")),
          regenerate: rect(visibleButton("重新生成图片")),
        },
        scrollArea: scrollArea ? { ...rect(scrollArea), clientHeight: scrollArea.clientHeight, scrollHeight: scrollArea.scrollHeight, scrollTop: scrollArea.scrollTop } : null,
        chapterRail: rect(document.querySelector('nav[aria-label="漫画章节"]')),
        pageRail: rect(document.querySelector('nav[aria-label="漫画页列表"]')),
        headers: [...document.querySelectorAll("header")].map((header) => ({ text: header.innerText.trim().slice(0, 64), ...rect(header) })),
      };
    }, `${comicPage} img`);
    assert(initial, "The selected comic image must be present before scrolling");
    if (width === 390) {
      assert(initial.visibleHeight >= initial.height - 1,
        `The complete generated image must fit inside the phone image viewport: ${JSON.stringify(initial)}`);
    }
    assert(initial.pageActions.edit && initial.pageActions.regenerate, "The page-level edit and regenerate actions must remain available");
    const actionsAfterImage = await browser.execute(() => innerWidth < 1280);
    if (actionsAfterImage) {
      assert(initial.pageActions.edit.top >= initial.bottom, "On narrow layouts, prompt editing follows the image preview");
      assert(initial.pageActions.regenerate.top >= initial.bottom, "On narrow layouts, image regeneration follows the image preview");
    } else {
      assert(initial.pageActions.edit.top < initial.top, "On desktop, prompt editing remains in the page header");
      assert(initial.pageActions.regenerate.top < initial.top, "On desktop, image regeneration remains in the page header");
    }
    const reachable = await reachByWheel(`${comicPage} img`, "y");
    assert.equal(reachable.reached, true, "The page image must be reachable through its visible scroll area");
    await browser.waitUntil(async () => browser.execute((selector) => {
      const image = document.querySelector(selector);
      return image?.complete === true && image.naturalWidth > 0;
    }, `${comicPage} img`), { timeout: 5000, timeoutMsg: "The selected comic image must decode after scrolling into view" });
    const decoded = await browser.execute((selector) => {
      const image = document.querySelector(selector);
      return { width: image.naturalWidth, height: image.naturalHeight };
    }, `${comicPage} img`);
    let pageActionsReachability = null;
    if (actionsAfterImage) {
      pageActionsReachability = await reachByWheel(await visibleButtonText("编辑提示词"), "y");
      assert.equal(pageActionsReachability.reached, true, "The page-level actions must be reachable after the image preview");
    }
    return { initialImageBounds: initial, imageReachable: reachable.reached, reachability: reachable, pageActionsReachability, decoded };
  }, widths: [1440, 1024, 900, 390], note: "首张截图保留未滚动首屏；首屏可见高度按真实滚动裁剪区测量，再验证图片和页级操作可达性。" },
  { id: "MC06", title: "漫画灯箱", task: "完整查看纵图、放大并重置后返回页内", open: async () => { await openComic(1, "image"); await browser.execute((selector) => { document.querySelector(selector)?.scrollIntoView({ block: "center" }); }, `${comicPage} img`); await $(`${comicPage} img`).click(); await $(dialog).waitForDisplayed(); }, scope: dialog, images: 1, expected: ["100%"], after: async (record, width) => {
    const geometry = () => browser.execute(() => {
      const image = document.querySelector('.a4n-image-lightbox-image');
      const stage = image.closest('.a4n-image-lightbox-stage').getBoundingClientRect();
      const body = image.closest('.a4n-image-lightbox-body').getBoundingClientRect();
      const box = image.getBoundingClientRect();
      const inside = bounds => box.left >= bounds.left - 1 && box.right <= bounds.right + 1 && box.top >= bounds.top - 1 && box.bottom <= bounds.bottom + 1;
      return {
        width: box.width, height: box.height,
        naturalWidth: image.naturalWidth, naturalHeight: image.naturalHeight,
        objectFit: getComputedStyle(image).objectFit,
        scale: new DOMMatrixReadOnly(getComputedStyle(image).transform).a,
        insideStage: inside(stage), insideBody: inside(body),
      };
    });
    const assertFit = value => {
      assert(value.naturalWidth > 0 && value.naturalHeight > 0, "The lightbox image must be decoded");
      assert(Math.abs(value.scale - 1) < 0.001, "The fitted view must be at 100% scale");
      assert.equal(value.insideStage, true, "The full portrait image must fit inside the stage at 100%");
      assert.equal(value.insideBody, true, "The modal body must not clip the portrait image at 100%");
      assert.equal(value.objectFit, "contain", "The image must keep its natural aspect ratio inside the stage");
    };
    const initial = await geometry();
    assertFit(initial);
    await $(`${dialog} button[aria-label="放大图片"]`).click();
    await browser.waitUntil(async () => (await geometry()).scale > 1.24, { timeout: 2500, timeoutMsg: "The image zoom button must visibly enlarge the image" });
    const zoomed = await geometry();
    assert(zoomed.width > initial.width && zoomed.height > initial.height);
    record.zoomed = await capture({ id: record.id, scope: dialog }, width, "-zoomed");
    await $(`${dialog} button[aria-label="重置图片视图"]`).click();
    await browser.waitUntil(async () => {
      const value = await geometry();
      return Math.abs(value.scale - 1) < 0.001 && value.insideStage && value.insideBody;
    }, { timeout: 2500, timeoutMsg: "Reset must restore the complete image inside the stage and modal body" });
    const reset = await geometry();
    assertFit(reset);
    assert(Math.abs(reset.width - initial.width) <= 1 && Math.abs(reset.height - initial.height) <= 1);
    record.reset = await capture({ id: record.id, scope: dialog }, width, "-reset");
    await browser.keys("Escape");
    await $(dialog).waitForExist({ reverse: true });
    await browser.waitUntil(async () => $(`${comicPage} img`).execute(element => element === document.activeElement), { timeout: 2500, timeoutMsg: "Closing the portrait preview must return focus to the original image" });
    return { initial, zoomed, reset, closedWithEscape: true, focusRestored: true };
  }, note: "几何检查覆盖舞台与Modal裁切边界和原比例；真实放大、重置及焦点返回，不以decoded代替完整显示。" },
  { id: "MC07", title: "漫画失败与续跑", task: "定位失败章节并识别恢复操作", open: () => openComic(4, "prompt", 2), expected: ["第4页", "继续"], note: "错误必须保留，续跑入口不能只剩颜色提示。" },
  { id: "MC08", title: "漫画末页可达", task: "通过横向轨道选择第6页并查看过期内容", open: () => openComic(6), expected: ["第一次选择", "需更新"], after: async () => {
    const selectedPage = await $('nav[aria-label="漫画页列表"] button[aria-current="true"]');
    assert((await selectedPage.getText()).includes("第 6 页"));
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(6).image_prompt);
    return { selectedPage: 6, promptId: 21006, currentTextMatches: true };
  }, note: "末页通过真实 wheel 后点击，核对当前页完整文字与过期状态。" },
  { id: "MC09", title: "漫画图片请求", task: "从页操作打开生成请求预览", open: async () => { await openComic(); await openMenu(`${comicPage} > header`, "查看生成图片请求"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["模型请求", "生成"], note: "预览按既有合同展示，不发送生成请求。" },
  { id: "MC10", title: "漫画优化指令", task: "为第3页输入多行优化指令并取消", open: async () => {
    await openComic(3);
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(3).image_prompt);
    await openMenu(comicPrompt, "优化提示词");
    await $(dialog).waitForDisplayed();
    const submit = await $(dialog).$('.//button[normalize-space()="开始优化"]');
    assert.equal(await submit.isEnabled(), false, "Empty required instructions must block submission");
    await $(`${dialog} textarea`).setValue("保留林岚右手的怀表。\n先展示钟面，再展示人物反应。");
    await browser.waitUntil(async () => submit.isEnabled());
    await focusTextEnd(`${dialog} textarea`);
  }, scope: dialog, expected: ["优化文字资产", "保留林岚右手的怀表", "先展示钟面"], after: async () => {
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    await $(dialog).waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("optimize_manga_prompt")), false);
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(3).image_prompt);
    return { cancelledBeforeModelCall: true, unchangedPromptId: 21003 };
  }, note: "真实输入多行指令、检查必填状态，再取消并核对原文不变。" },
  { id: "MC11", title: "漫画删除确认", task: "查看整页提示词删除范围并安全取消", open: async () => {
    await openComic(3);
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(3).image_prompt);
    await openMenu(`${comicPage} > header`, "删除当前页提示词");
    await $(dialog).waitForDisplayed();
  }, scope: dialog, expected: ["删除第 3 页提示词", "取消"], after: async () => {
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    await $(dialog).waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("delete_manga_prompt")), false);
    assert.equal((await $$('nav[aria-label="漫画页列表"] button')).length, 6);
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(3).image_prompt);
    return { cancelledBeforeDeletion: true, unchangedPromptId: 21003, pages: 6 };
  }, note: "取消按钮真实可用；当前第3页全文及六页列表均保留。" },
  { id: "MC13", title: "漫画跨页草稿", task: "切换页面后保留提示词阅读方式和编辑内容", open: async () => {
    await openComic(3);
    await openMenu(comicPrompt, "编辑提示词");
    assert.equal(await $(`${comicPrompt} textarea`).getValue(), comicFixture(3).image_prompt);
    await $(`${comicPrompt} textarea`).setValue(`${comicFixture(3).image_prompt}\n第3页保留的审查草稿`);
    await selectComicPage(5);
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(5).image_prompt, "Switching pages must retain the prompt view and display the selected page's full text");
    await selectComicPage(3);
    assert.equal(await $(`${comicPrompt} textarea`).getValue(), `${comicFixture(3).image_prompt}\n第3页保留的审查草稿`);
    await focusTextEnd(`${comicPrompt} textarea`);
  }, scope: comicPage, expected: ["第 3 页", "保存"], after: async () => {
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("update_manga_prompt")), false);
    return { draftPromptId: 21003, comparedPromptId: 21005, fullTextRetained: true };
  }, note: "比较两页完整正文，真实返回草稿并将光标移到末行；不重新输入或提交。" },
  { id: "MC14", title: "漫画保存中切页", task: "保存第3页时切到第5页，响应只清理原页草稿", open: async () => {
    await openComic(5); await openMenu(comicPrompt, "编辑提示词"); await $(`${comicPrompt} textarea`).setValue(`${comicFixture(5).image_prompt}\n第5页独立草稿`);
    await selectComicPage(3); await openMenu(comicPrompt, "编辑提示词"); await $(`${comicPrompt} textarea`).setValue(`${comicFixture(3).image_prompt}\n第3页已提交草稿`);
    await browser.execute(() => { window.__uiAudit.deferred.push("update_manga_prompt"); });
    await $(comicPrompt).$('.//button[normalize-space()="保存"]').click();
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.pending.some(item => item.command === "update_manga_prompt")));
    await selectComicPage(5);
    await browser.execute(expectedText => {
      const pending = window.__uiAudit.pending.find(item => item.command === "update_manga_prompt");
      const input = pending.body.input;
      if (input.prompt_id !== 21003) throw new Error("The save request was redirected to the newly selected page");
      if (input.image_prompt !== expectedText) throw new Error("The save request did not carry the original page's complete draft");
      const row = window.__uiAudit.fixtures.byChapter[1].list_manga_page_prompts.find(item => item.id === input.prompt_id);
      row.image_prompt = input.image_prompt;
      row.drawing_prompt.image_prompt = input.image_prompt;
      pending.resolve(Response.json(row));
    }, `${comicFixture(3).image_prompt}\n第3页已提交草稿`);
    await browser.waitUntil(async () => (await $(`${comicPrompt} textarea`).isEnabled()) === true);
    assert.equal(await $(`${comicPrompt} textarea`).getValue(), `${comicFixture(5).image_prompt}\n第5页独立草稿`, "The completed save must preserve the other page's entire draft");
    await focusTextEnd(`${comicPrompt} textarea`);
  }, scope: comicPage, expected: ["第 5 页", "保存"], after: async () => {
    await selectComicPage(3);
    assert.equal(await $(`${comicPrompt} pre`).getText(), `${comicFixture(3).image_prompt}\n第3页已提交草稿`);
    const result = await browser.execute(() => ({
      savedIds: window.__uiAudit.requests.filter(item => item.command === "update_manga_prompt").map(item => item.body.input.prompt_id),
      untouchedText: window.__uiAudit.fixtures.byChapter[1].list_manga_page_prompts.find(item => item.id === 21005).image_prompt,
    }));
    assert.deepEqual(result.savedIds, [21003]);
    assert.equal(result.untouchedText, comicFixture(5).image_prompt);
    return { savedPromptId: 21003, preservedPromptId: 21005, fullTextsMatch: true };
  }, note: "精准延迟单个保存，核对请求全文/目标、返回全文和其他页未提交的独立草稿。" },

  { id: "MV01", title: "视频素材", task: "核对场景和物体的正常、缺失和过期素材", open: openVideoMaterials, count: ["article[data-state]", 6], expected: ["旧书店", "旧怀表"], note: "素材主操作是否藏在默认折叠的生成设置内。" },
  { id: "MV02", title: "素材生成设置", task: "查看尺寸、风格及批量补全入口", open: async () => { await openVideoMaterials(); await $('//summary[.//span[normalize-space()="生成设置"]]').click(); }, expected: ["补齐", "缺失"], note: "尺寸与风格可折叠，主要补全任务应易发现。" },
  { id: "MV03", title: "素材缺失筛选", task: "用已有筛选仅查看缺失素材", open: async () => { await openVideoMaterials(); const trigger = await $('//label[.//span[normalize-space()="状态"]]//button[@role="combobox"]'); await selectOption(trigger, "缺失"); }, count: ["article[data-state]", 2], note: "真实筛选结果应改变，不用相同数组冒充筛选成功。" },
  { id: "MV04", title: "素材上传对话框", task: "查看上传前的角色/场景归属与主图开关", open: async () => { await openVideoMaterials(); await openMenu('article[data-state="ready"]', "上传素材图"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["上传", "主"], note: "保留文件选择和上传后设为主图；本例不上传文件。" },
  ...["概览", "来源", "管理"].map((tab, index) => ({ id: `MV0${5 + index}`, title: `素材图${tab}`, task: `打开图片详情的${tab}模式`, open: () => openMaterialDetails(tab), scope: dialog, expected: tab === "来源" ? ["书店素材图 · UI样本", "旧书店场景参考图"] : [tab], images: tab === "概览" ? 1 : 0, note: "三种详情模式独立截图，来源必须展示实际元数据，不能由空提示替代。" })),
  { id: "MV08", title: "片段规划", task: "逐段阅读场景、动作与调度摘要", open: () => openVideo("片段规划"), count: ['section[aria-label="视频片段规划"] ol > li', 6], expected: ["街口雨幕", "后门敲响"], note: "摘要与调度被截断时，应有完整阅读路径。" },
  { id: "MV09", title: "规划参数", task: "查看比例、语言、时长候选和公共风格", open: async () => { await openVideo("片段规划"); await $('//summary[.//span[normalize-space()="规划配置"]]').click(); }, expected: ["候选", "风格"], note: "三列参数在390px下必须保持标签和控件可读。" },
  { id: "MV10", title: "规划编辑", task: "进入完整片段编辑并展开一段演出参数", open: async () => { await openVideo("片段规划"); await buttonText("编辑规划").click(); await $('section[aria-label="视频片段规划"] fieldset > details > summary').click(); }, expected: ["保存规划", "取消编辑"], note: "保留原完整VideoSegmentPlan合同；本例不提交修改。" },
  { id: "MV11", title: "片段生成总览", task: "从混合状态的六段计划选择当前片段", open: async () => { await openVideo(); await selectSegment(1); }, count: ["button[data-video-segment]", 6], expected: ["片段", "首尾帧"], note: "原生段与插入段、采纳/失败/待配状态须可区分。" },
  ...[
    ["MV12", 1, "首尾帧", "生成方案与已准备首尾帧"],
    ["MV13", 2, "首帧续接", "更早已采纳视频的尾帧续接"],
    ["MV14", 3, "分镜", "分镜板与视频失败状态"],
    ["MV15", 4, "参考媒体", "图片、视频和音频参考"],
    ["MV16", 5, "Chat", "复制Chat提示词的手工流程"],
    ["MV17", 6, "首尾帧", "尚未准备输入时的下一步"],
  ].map(([id, number, mode, task]) => ({
    id, title: `片段${number}·${mode}`, task, open: () => openSegment(number), scope: dialog, expected: [mode],
    after: async (record) => {
      if (id !== "MV17") return inspectVideoEditorEnd(record);
      const reason = "请先准备当前片段的首尾帧提示词";
      assert.equal(record.text.split(reason).length - 1, 1, "The generation workspace must show the blocking reason once");
      assert.equal(await $(dialog).$('.//button[normalize-space()="生成最终提示词"]').isEnabled(), false);
      return { blockingReason: reason, generationDisabled: true };
    },
    note: "五种就绪模式检查末端字段；未准备模式保留唯一原因与禁用生成。",
  })),
  { id: "MV18", title: "首帧配置", task: "查看已生成首帧及文字操作", open: () => openFrame(1, "首帧"), scope: dialog, expected: ["首帧配置", "保存", "生成图片"], images: 1, note: "底栏可能有六个动作，390px需检查正文剩余高度。" },
  { id: "MV19", title: "尾帧配置", task: "查看尾帧，区分当前片段起止状态", open: () => openFrame(1, "尾帧"), scope: dialog, expected: ["尾帧配置"], images: 1, note: "必须是真实尾帧内容，不能复用空配置截图。" },
  { id: "MV20", title: "连续性来源", task: "辨认续接尾帧与更早视频的关系", open: async () => { await openSegment(2); await $(`${dialog} button[aria-label="配置续接首帧"]`).$('.//span[normalize-space()="配置续接首帧"]').click(); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "续接首帧配置"); }, scope: dialog, expected: ["来源视频"], images: 1, note: "来源ID应配合片段信息和尾帧预览，不能要求记忆裸编号。" },
  { id: "MV21", title: "分镜板配置", task: "查看分镜板及已有生成提示词", open: () => openFrame(3, "分镜板"), scope: dialog, expected: ["分镜板配置"], images: 1, note: "保留生成、导入、删除与请求预览入口。" },
  { id: "MV22", title: "参考图管理", task: "从片段配置进入素材库选择", open: async () => { await openSegment(1); await $(`${dialog} button[aria-label="添加参考图或素材"]`).click(); await $(`${dialog} [aria-label="参考素材筛选"]`).waitForDisplayed(); }, scope: dialog, expected: ["素材", "参考"], images: 1, note: "图像预览和选中/加入素材库是不同操作，需同时可达。" },
  { id: "MV23", title: "视频结果播放", task: "播放已采纳视频并识别当前记录", open: openVideoResult, scope: resultPane, expected: ["已采用"], after: () => provePlayback(`${resultPane} video`), note: "必须解码且播放时间推进，服务启动或video标签不算成功。" },
  { id: "MV24", title: "视频提示词查看", task: "阅读当前成片的完整提示词", open: () => openVideoPrompt(false), scope: dialog, expected: ["视频提示词"], after: async record => { const source = fixtures.byChapter[1].list_videos_by_chapter.find(video => video.id === 31001); assert(record.text.includes(source.prompt), "The dialog must display the selected video's exact stored prompt"); return { videoId: source.id, promptMatchesRecord: true }; }, note: "对照所选视频的完整保存文本，不能把最终提示词记录当成视频快照。" },
  { id: "MV25", title: "视频提示词编辑", task: "查看长提示词与负面提示词编辑及保存位置", open: () => openVideoPrompt(true), scope: dialog, expected: ["保存记录"], count: [`${dialog} textarea`, 2], note: "检查保存按钮位于固定底栏，长文字仍可完整滚动。" },
  { id: "MV26", title: "视频请求预览", task: "从成片菜单查看对应模型请求", open: async () => { await openVideoResult(); await openMenu(`${resultPane} [data-video-preview-id="31001"]`, "查看模型请求"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["请求"], note: "请求来自合成快照，没有真实凭证或对外调用。" },
  { id: "MV27", title: "创建插入片段", task: "在既有片段间准备插入内容", open: openInserted, scope: dialog, expected: ["生成插入片段", "参考"], note: "清楚区分生成描述与确认插入，不在审查中生成。" },
  { id: "MV28", title: "插入参考图选择", task: "点击图片主体选中参考图", open: openInserted, scope: dialog, expected: ["生成插入片段"], after: async () => { const option = await $(`${dialog} button:has(img)`); await option.$("img").click(); await browser.waitUntil(async () => (await option.getAttribute("class")).includes("ring-1"), { timeout: 2500, timeoutMsg: "Clicking the reference image must select it, not replace the task with a lightbox" }); assert.equal(await $(`${dialog} h3`).getText(), "生成插入片段"); return { imageClickSelectedReference: true }; }, note: "图片主体选择，独立预览按钮放大；两种操作不能吞事件。" },
  { id: "MV29", title: "插入片段详情", task: "查看插入位置、提示词与移动/删除动作", open: async () => { await openVideo(); await $('//button[.//div[normalize-space()="信封暗记特写"] and not(ancestor::*[@role="dialog"])]').click(); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["插入片段详情", "选择新的位置"], after: async () => { const location = await $(`${dialog} button[role="combobox"]`); assert.equal(await location.isEnabled(), true); await location.click(); const options = await $$('[role="option"]'); assert(options.length > 2, "An inserted segment must expose real alternative positions"); await browser.keys("Escape"); return { moveOptions: options.length }; }, note: "保留位置编辑与已有视频操作，不执行删除。" },
  { id: "MV30", title: "成片合成", task: "审查四个可用片段的顺序与已推送前缀", open: async () => {
    await openVideo("成片合成");
    const video = ".a4n-page-container video";
    await waitForPlayable(video);
    await browser.waitUntil(async () => $(video).execute(element => element.readyState >= 3), {
      timeout: 10000, timeoutMsg: "The preview must be ready to play before the loading state clears",
    });
  }, expected: ["成片预览", "片段顺序"], count: ['[role="region"][aria-label="片段顺序"] ul > li', 4], after: async (_record, width) => {
    const playback = await provePlayback(".a4n-page-container video");
    const clipOrder = await $('[role="region"][aria-label="片段顺序"]');
    const lastClip = await clipOrder.$("ul > li:last-child");
    const scrollMetrics = await clipOrder.execute(element => ({
      clientHeight: element.clientHeight,
      scrollHeight: element.scrollHeight,
      scrollTop: element.scrollTop,
      keyboardFocusable: element.tabIndex >= 0,
      scrollbarWidth: getComputedStyle(element).scrollbarWidth,
    }));
    assert.equal(scrollMetrics.keyboardFocusable, true);
    const previewHeight = await $(".a4n-video-assembly-preview-media").getSize("height");
    if (width < 1280) {
      assert(scrollMetrics.scrollHeight <= scrollMetrics.clientHeight + 1, "Stacked clip lists must flow with the page instead of nesting a scroller");
      assert(previewHeight > 100, "The stacked preview must retain a visible media frame");
    }
    const previewLoading = await $(".a4n-video-assembly-preview [role=\"status\"]");
    assert.equal(await previewLoading.isDisplayed(), false, "The loading state must clear after media data is ready");
    let keyboardScrollTop = null;
    if (width === 390) {
      await clipOrder.execute(element => element.focus());
      await browser.keys("PageDown");
      const assemblyPanel = await $(".a4n-video-assembly-tabpanel");
      await browser.waitUntil(async () => (await assemblyPanel.getProperty("scrollTop")) > 32, { timeout: 2500 });
      keyboardScrollTop = await assemblyPanel.getProperty("scrollTop");
      assert(keyboardScrollTop > 32, "Keyboard users must be able to scroll the clip order");
    }
    const reachability = await reachByWheel(lastClip, "y");
    assert.equal(reachability.reached, true, "The last assembly clip must be reachable by scrolling");
    if (width === 390) {
      assert(reachability.scrollable, "The last phone clip must be reachable through page scrolling");
      assert(!String(reachability.scrollTarget).includes("a4n-video-assembly-clip-list"), "The phone order list must not create a nested scroller");
    }
    return { playback, lastClipReachability: reachability, scrollMetrics, keyboardScrollTop };
  }, note: "预览保留真实媒体解码；手机端顺序区检查真实滚动、末项与键盘可达性。" },
  { id: "MV31", title: "后处理批次", task: "从两批成片进入待确认的编辑版本", open: () => openPostprocess(0), expected: ["推送批次", "后处理预览", "剪辑条"], count: ["[data-video-push-id]", 2], note: "播放器不应把实际编辑时间条压到不可见处。" },
  { id: "MV32", title: "后处理插入点", task: "点击时间条设置一个插入点", open: () => openPostprocess(1), expected: ["上传并插入"], count: [`${timeline} [role="slider"]`, 1], note: "命中区、时间读数和撤销选择入口需明确。" },
  { id: "MV33", title: "后处理选区", task: "用两个标记定义可替换的片段", open: () => openPostprocess(2), expected: ["上传并替换"], count: [`${timeline} [role="slider"]`, 2], note: "检查双标记间距、选区高亮和长时间读数的碰撞。" },
  { id: "MV34", title: "后处理键盘微调", task: "通过方向键精确移动时间标记", open: () => openPostprocess(1), expected: ["剪辑条"], after: async () => { const marker = await $(`${timeline} [role="slider"]`); const before = Number(await marker.getAttribute("aria-valuenow")); await marker.execute(element => element.focus()); await browser.keys("ArrowRight"); const after = Number(await marker.getAttribute("aria-valuenow")); assert(after > before, "ArrowRight must move the selected marker"); return { before, after }; }, note: "保留0.05秒方向键微调与大步移动能力。" },
  { id: "MV35", title: "帧图点击配置", task: "点击首帧图片中央进入配置", open: async () => { await openSegment(1); await $(`${dialog} button[aria-label="配置首帧"] img`).click(); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "首帧配置", { timeout: 2500, timeoutMsg: "The frame image must activate configuration" }); }, scope: dialog, expected: ["首帧配置"], images: 1, note: "图片主体属于配置入口，不能被预览组件截断。" },
  { id: "MV36", title: "帧图独立预览", task: "用独立预览按钮放大首帧并返回配置页", open: async () => { await openSegment(1); await $(`${dialog} button[aria-label="图片预览"]`).click(); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["100%"], images: 1, after: async () => { await browser.keys("Escape"); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "片段 1 · 配置"); return { returnedToSegmentConfig: true }; }, note: "独立放大保留，关闭只退出顶部灯箱。" },
  { id: "MV37", title: "插入参考图独立预览", task: "放大参考图时保持原选择状态", open: openInserted, scope: dialog, expected: ["生成插入片段"], after: async (record, width) => { const option = await $(`${dialog} button:has(img)`); const selected = (await option.getAttribute("class")).includes("ring-1"); await $(`${dialog} button[aria-label="图片预览"]`).click(); await browser.waitUntil(async () => (await $(dialog).getText()).includes("100%")); record.lightbox = await capture({ id: "MV37", scope: dialog }, width, "-lightbox"); await browser.keys("Escape"); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "生成插入片段"); assert.equal((await option.getAttribute("class")).includes("ring-1"), selected); return { selectionUnchanged: true }; }, note: "放大与选择分别验证，不能以灯箱打开替代选择成功。" },

  { id: "MG01", title: "Galgame场次与恢复", task: "查看三个场次、来源与失败恢复", open: () => openGalgame("scenes"), expected: ["铜铃之后的来信", "纸背上的另一条路", "第三站台的约定"], after: async () => {
    const firstSceneRow = '//div[contains(@class,"divide-y")]/button[contains(., "铜铃之后的来信")]';
    const reachability = await reachByWheel(firstSceneRow);
    assert.equal(reachability.reached, true, "The first scene row must be visible through real scrolling");
    const rowText = await $(firstSceneRow).getText();
    assert(rowText.includes("主线") && rowText.includes("场次 3"), "The transition must remain readable beside its scene kind");
    const arrow = await $(firstSceneRow).$('svg.lucide-arrow-right');
    assert.equal(await arrow.isExisting(), true, "Scene transitions must use a Lucide icon");
    assert.equal(await arrow.getAttribute("aria-hidden"), "true", "Decorative transition icons must be hidden from assistive technology");
    return { transitionIcons: 1, decorativeIconsHidden: true, firstSceneRow: rowText, reachability };
  }, note: "场次长列表和状态面板不能淹没素材制作入口。" },
  { id: "MG02", title: "Galgame混合台词", task: "阅读八行不同类型的剧本", open: () => openGalgame(), count: [galgameLines, 8], expected: ["林岚", "周衡"], after: async (_record, width) => {
    const disclosure = await $(`${galgameEditor} details[data-scene-meta-disclosure]`);
    const summary = await disclosure.$("summary");
    const firstLine = await $(`${galgameLines}:first-of-type`);
    const open = await disclosure.getProperty("open");
    const firstLineRect = await firstLine.execute(element => {
      const { top, bottom } = element.getBoundingClientRect();
      return { top, bottom, viewportHeight: innerHeight };
    });
    const body = await firstLine.$("textarea");
    const bodyRect = await body.execute(element => {
      const { top, bottom } = element.getBoundingClientRect();
      return { top, bottom, viewportHeight: innerHeight };
    });
    let keyboardDisclosureToggle = false;
    let summaryFocus;
    if (width < 768) {
      assert.equal(await summary.isDisplayed(), true, "Mobile scene metadata needs a visible disclosure trigger");
      assert.equal(open, false, "Mobile scene metadata should start collapsed so the script leads");
      assert(bodyRect.top >= 0 && bodyRect.bottom <= bodyRect.viewportHeight, `The first script body must be fully visible on the phone: ${JSON.stringify(bodyRect)}`);
      const activeView = await $('[aria-label="GALGame 移动工作区"] button[aria-pressed="true"]');
      await activeView.execute(element => element.focus());
      await browser.keys("Tab");
      summaryFocus = await summary.execute(element => {
        const style = getComputedStyle(element);
        return {
          active: document.activeElement === element,
          visible: element.matches(":focus-visible"),
          outlineStyle: style.outlineStyle,
          outlineWidth: style.outlineWidth,
        };
      });
      assert.equal(summaryFocus.active, true, `Tab must move from the mobile workspace switch to scene information: ${JSON.stringify(summaryFocus)}`);
      assert.equal(summaryFocus.visible, true, "Keyboard focus must use the visible focus-visible state");
      assert.equal(summaryFocus.outlineStyle, "solid");
      assert.equal(summaryFocus.outlineWidth, "2px");
      await browser.keys("Enter");
      await browser.waitUntil(async () => (await disclosure.getProperty("open")) === true);
      await ready();
      assert.equal(await disclosure.getProperty("open"), true, "React must retain the keyboard-expanded disclosure state");
      await browser.keys("Enter");
      await browser.waitUntil(async () => !(await disclosure.getProperty("open")));
      keyboardDisclosureToggle = true;
    } else {
      assert.equal(open, true, "Wide Galgame layouts keep scene metadata expanded");
    }
    return { sceneMetaOpen: open, sceneMetaSummaryVisible: await summary.isDisplayed(), firstLine: firstLineRect, firstLineBody: bodyRect, keyboardDisclosureToggle, summaryFocus: width < 768 ? summaryFocus : undefined };
  }, note: "重复的姿态、表情、CG和转场控件是否打断台词阅读；窄屏先呈现首行，场景信息仍可展开。" },
  { id: "MG03", title: "Galgame背景绑定", task: "核对场景背景与既有素材绑定入口", open: async () => {
    await openGalgame();
    const sceneMeta = await $(`${galgameEditor} details[data-scene-meta-disclosure]`);
    if (!(await sceneMeta.getProperty("open"))) await sceneMeta.$("summary").click();
    await openDetails(`${galgameEditor} > div:nth-child(2) details:not([data-scene-meta-disclosure])`);
  }, scope: galgameEditor, images: 1, expected: ["背景"], note: "背景、当前素材与编号输入应有清楚关联。" },
  { id: "MG04", title: "Galgame对白", task: "查看说话角色、台词与姿态表情", open: () => openGalgamePerformance(2), tail: `${galgameLines}:nth-of-type(2) button[aria-label="表情"]`, expected: ["林岚", "表情"], note: "演出默认收起；实际展开后验证持久字段名与当前值。" },
  { id: "MG05", title: "Galgame独白与演出", task: "对照独白和舞台指示的编辑差异", open: () => openGalgame(), tail: `${galgameLines}:nth-of-type(5) textarea`, expected: ["独白", "演出"], note: "保留五种行类型各自字段，不以同一个空行代表。" },
  { id: "MG06", title: "Galgame末行选择支", task: "滚到第8行确认分支目标与合流", open: () => openGalgame(), tail: '//section[@aria-label="编辑"]//article[8]//button[normalize-space()="保存选择支"]', expected: ["保存选择支"], note: "900px是外层与内层滚动接管的重点断点。" },
  { id: "MG07", title: "GalgameCG与转场", task: "查看启用CG的台词及演出说明", open: () => openGalgamePerformance(6), tail: '//section[@aria-label="编辑"]//article[6]//textarea[@placeholder="事件 CG 画面描述"]', expected: ["CG"], note: "实际展开演出；已启用CG保留默认展开，正文仍是视觉主体。" },
  { id: "MG08", title: "Galgame音频媒体", task: "展开台词媒体并播放已有BGM", open: async () => {
    await openGalgame("editor", { openSceneProjection: false });
    await openDetails('div[role="tabpanel"] > details.a4n-flat-surface');
    const mobileWorkspace = await $('[aria-label="GALGame 移动工作区"]');
    if (await mobileWorkspace.isDisplayed()) {
      await mobileWorkspace.$('.//button[normalize-space()="编辑"]').click();
      await ready();
    }
    await openDetails(`${galgameLines}:first-of-type > details:not([class*="group/performance"])`);
  }, scope: galgameEditor, expected: ["BGM", "SFX"], after: async (record, width) => {
    const audio = `${galgameLines}:first-of-type audio`;
    const firstLine = `${galgameLines}:first-of-type`;
    const playerRow = `${firstLine} div.grid:has(> audio)`;
    const bgmImport = await $(firstLine).$('.//button[normalize-space()="导入 BGM"]');
    const sfxImport = await $(firstLine).$('.//button[normalize-space()="导入 SFX"]');
    const order = await browser.execute((lineSelector) => {
      const line = document.querySelector(lineSelector);
      const rect = (element) => {
        const bounds = element?.getBoundingClientRect();
        return bounds ? { top: bounds.top, bottom: bounds.bottom } : null;
      };
      const audio = line?.querySelector("audio");
      const buttons = [...(line?.querySelectorAll("button") ?? [])];
      return {
        player: rect(audio?.closest(".grid")),
        bgmImport: rect(buttons.find((button) => button.textContent.trim() === "导入 BGM")),
        sfxImport: rect(buttons.find((button) => button.textContent.trim() === "导入 SFX")),
      };
    }, firstLine);
    const sfxReachability = await reachByWheel(sfxImport);
    assert.equal(sfxReachability.reached, true, "The SFX import control must remain reachable after the existing player");
    record.importControls = await capture({ id: "MG08", scope: galgameEditor }, width, "-import-controls");
    const viewport = await browser.execute(() => ({ width: innerWidth, height: innerHeight }));
    const playerReachability = await reachByWheel(audio);
    const playerRect = await browser.execute((selector) => {
      const { x, y, width, height } = document.querySelector(selector).getBoundingClientRect();
      return { x, y, width, height };
    }, audio);
    const playerLayout = await browser.execute((selector, viewportWidth) => {
      const row = document.querySelector(selector);
      const rect = (element) => {
        const bounds = element?.getBoundingClientRect();
        return bounds ? { left: bounds.left, right: bounds.right, top: bounds.top, bottom: bounds.bottom } : null;
      };
      return {
        width: viewportWidth,
        row: rect(row),
        label: rect(row?.querySelector("span")),
        player: rect(row?.querySelector("audio")),
        unbind: rect(row?.querySelector("button")),
      };
    }, playerRow, width);
    assert.equal(playerReachability.reached, true,
      `The complete BGM player must be reachable before playback: ${JSON.stringify({ playerReachability, playerRect, viewport })}`);
    assert(playerRect.width > 0 && playerRect.height > 0, "The native BGM player must have a visible rectangle");
    assert(playerRect.x >= 0 && playerRect.y >= 0 && playerRect.x + playerRect.width <= viewport.width && playerRect.y + playerRect.height <= viewport.height,
      `The complete native BGM player must be within the viewport: ${JSON.stringify({ playerRect, viewport })}`);
    assert(order.player && order.bgmImport && order.sfxImport, "The player and both audio import controls must render");
    assert(order.player.bottom <= order.bgmImport.top && order.player.bottom <= order.sfxImport.top,
      `Existing audio must precede import controls: ${JSON.stringify(order)}`);
    assert(playerLayout.row && playerLayout.label && playerLayout.player && playerLayout.unbind,
      `The bound BGM row must include its label, player, and unbind control: ${JSON.stringify(playerLayout)}`);
    if (playerLayout.width < 768) {
      assert(playerLayout.label.bottom <= playerLayout.player.top
        && playerLayout.unbind.bottom <= playerLayout.player.top,
      `The phone player must sit beneath its label and unbind action: ${JSON.stringify(playerLayout)}`);
      assert(playerLayout.row.left <= playerLayout.player.left + 1
        && playerLayout.player.right >= playerLayout.row.right - 1
        && playerLayout.label.right <= playerLayout.unbind.left,
      `The phone player must use the row width while the unbind action stays beside its label: ${JSON.stringify(playerLayout)}`);
    } else {
      assert(playerLayout.row.left <= playerLayout.label.left
        && playerLayout.label.right <= playerLayout.player.left
        && playerLayout.player.right <= playerLayout.unbind.left
        && playerLayout.unbind.right <= playerLayout.row.right,
      `Desktop BGM controls must stay aligned in one row: ${JSON.stringify(playerLayout)}`);
    }
    return {
      playerReachability,
      playerRect,
      playerLayout,
      playerBeforeImportControls: true,
      sfxReachability,
      viewport,
      playback: await provePlayback(audio),
    };
  }, widths: [1440, 1024, 900, 390], note: "先真实滚到BGM控制区再验证播放时间推进，保留可见音频控件的interaction截图。" },
  { id: "MG09", title: "Galgame视觉资产计划", task: "查看6背景、12立绘单元和2CG", open: openGalgameAssets, scope: 'section[aria-label="资产规划"]', expected: ["角色差分", "CG", "站台·镜中出口"], count: ['section[aria-label="资产规划"] tbody td', 12], after: async () => { assert.equal(await $$('//section[@aria-label="资产规划"]//tbody//button[normalize-space()="生成素材"]').length, 0); return { perAssetActionsHiddenDuringPlanning: true }; }, note: "规划阶段只承担清单与状态核对，逐项生成留在素材生成阶段。" },
  { id: "MG10", title: "Galgame矩阵末项", task: "找到矩阵最右列并读取生成状态", open: async () => { await openGalgameAssets(); await chooseGalgameStage("asset_generation", "素材生成"); const target = 'section[aria-label="资产规划"] tbody tr:last-child td:last-child button'; await browser.waitUntil(async () => (await $$(target)).length === 1, { timeout: 2500, timeoutMsg: "Per-asset generation actions must appear in the generation stage" }); await reachByWheel(target, "y"); }, scope: 'section[aria-label="资产规划"]', tail: 'section[aria-label="资产规划"] tbody tr:last-child td:last-child button', axis: "x", expected: ["视觉资产", "生成素材"], note: "逐项生成只在素材生成阶段出现；窄屏保留真实横向滚动与大图预览。" },
  { id: "MG11", title: "Galgame生成选项", task: "区分断点续跑与指定阶段重新生成", open: async () => { await openGalgameScriptControls(); await openDetails('details[aria-label="剧本生成"]'); await openDetails('details[aria-label="剧本生成"] details'); }, widths: [1440, 1024, 900, 390], expected: ["继续生成", "自动选择起点"], after: async () => {
    const options = await $('details[aria-label="剧本生成"]');
    const primary = await options.$("button.w-full");
    assert.equal((await primary.getText()).trim(), "继续生成");
    await options.$('button[role="combobox"]').click();
    await $('//*[@role="option" and normalize-space()="资产意图"]').click();
    assert.equal((await primary.getText()).trim(), "从资产意图重新生成");
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("start_galgame_pipeline")), false);
    return { selectedStage: "asset_generation", operationLabel: "从资产意图重新生成", submitted: false };
  }, note: "未指定阶段时由续跑/生成操作决定起点；指定阶段后主按钮准确说明重生成范围，原请求参数不变。" },
  { id: "MG12", title: "Galgame请求预览", task: "从生成选项查看请求并确认其不会触发生成", open: async () => { await openGalgameScriptControls(); await openDetails('details[aria-label="剧本生成"]'); await openMenu('details[aria-label="剧本生成"]', "查看模型请求"); await $(dialog).waitForDisplayed(); }, widths: [1440, 1024, 900, 390], scope: dialog, expected: ["模型请求预览", "不会发送模型请求", "模型", "模板", "实际提示内容", "完整请求", "上下文详情", "复制全部"], after: async () => {
    const commands = await browser.execute(() => window.__uiAudit.commands);
    assert(commands.includes("preview_galgame_pipeline_request"));
    assert.equal(commands.includes("start_galgame_pipeline"), false);
    return { previewOnly: true };
  }, note: "读请求正文与模型/模板来源；请求预览只读，不发起生成。" },
  { id: "MG13", title: "Galgame重置确认", task: "核对删除与保留范围，并安全取消", frequency: "低", widths: [1440, 1024, 900, 390], open: async () => { await openGalgame(); await openMenu(".a4n-page-header", "重置生成数据"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["不可恢复", "剧本", "视觉生成设置", "已生成媒体", "来源内容", "来源同步关系", "模型配置", "重置"], after: async (_record, width) => {
    const confirm = await $(dialog);
    const cancelButton = await confirm.$('.//button[normalize-space()="取消"]');
    const resetButton = await confirm.$('.//button[normalize-space()="重置"]');
    assert.equal(await cancelButton.isDisplayed(), true, "The destructive confirmation must expose a visible cancel action");
    assert.equal(await resetButton.isDisplayed(), true, "The destructive action must remain explicit");
    assert.equal(await $(`${dialog} button[aria-label="关闭"]`).isDisplayed(), true);
    const cancelPosition = await cancelButton.getLocation();
    const resetPosition = await resetButton.getLocation();
    if (width < 640) assert(cancelPosition.y < resetPosition.y, "Mobile must place cancel above the destructive action");
    else assert(cancelPosition.x < resetPosition.x, "Desktop must place cancel before the destructive action");
    await cancelButton.click();
    await $(dialog).waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("reset_project_generated_data")), false);

    await openMenu(".a4n-page-header", "重置生成数据");
    await $(dialog).waitForDisplayed();
    await browser.keys("Escape");
    await $(dialog).waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("reset_project_generated_data")), false);
    return { visibleCancelAction: true, cancelPrecedesReset: true, cancelledWithButton: true, escapeCloses: true, cancelledBeforeMutation: true };
  }, note: "核对删除/保留范围，实际使用取消按钮与 Escape 退出；不触发重置。" },
  { id: "MG14", title: "Galgame剧情图连线方向", task: "选中一条连线并核对来源、去向和读屏方向", widths: [1440, 900, 390], open: async () => {
    const graph = {
      branch_groups: [],
      edges: [{ branch_group_id: null, display_order: 1, from_node_id: "source-node", from_port: "out", guard: null, id: "graph-edge-1", kind: "sequence", state_operations: [], to_node_id: "target-node", to_port: "in" }],
      entry_node_id: "source-node",
      id: "graph-direction-review",
      loop_groups: [],
      nodes: [
        { id: "source-node", kind: "narrative", layout_x: 0, layout_y: 0, lines: [], media_intents: [], primary_source: null, source_refs: [], state_after: {}, state_before: {}, state_operations: [], summary: "剧情起点", title: "剧情起点" },
        { id: "target-node", kind: "terminal", layout_x: 360, layout_y: 0, lines: [], media_intents: [], primary_source: null, source_refs: [], state_after: {}, state_before: {}, state_operations: [], summary: "剧情终点", title: "剧情终点" },
      ],
      project_id: 5,
      revision: 1,
      schema_version: "galgame_story_graph_v1",
      source_snapshot_id: 50,
      state_variables: [],
      terminal_node_ids: ["target-node"],
    };
    await installStoryGraphReviewFixture(graph);
    await openGalgame("editor", { openSceneProjection: false });
    if ((await browser.execute(() => innerWidth)) < 1024) {
      const graphPane = await $('[aria-label="节点工作区"]');
      const graphButton = await graphPane.$('.//button[normalize-space()="图谱"]');
      assert.equal((await reachByWheel(graphButton)).reached, true);
      await graphButton.click();
      await ready();
    }
    const edgeSelector = '.react-flow__edge[data-id="graph-edge-1"]';
    const edge = await $(edgeSelector);
    await edge.waitForDisplayed();
    assert.equal((await reachByWheel(edge)).reached, true, "The graph connection must be visible and pointer-reachable");
    const point = await browser.execute((selector) => {
      const path = document.querySelector(`${selector} .react-flow__edge-interaction`);
      const group = path?.closest(".react-flow__edge");
      const matrix = path?.getScreenCTM();
      const length = path?.getTotalLength() ?? 0;
      if (!path || !group || !matrix || length <= 0) return null;
      for (const fraction of [0.5, 0.25, 0.75, 0.1, 0.9]) {
        const local = path.getPointAtLength(length * fraction);
        const screen = new DOMPoint(local.x, local.y).matrixTransform(matrix);
        const x = Math.round(screen.x), y = Math.round(screen.y);
        if (x < 0 || y < 0 || x >= innerWidth || y >= innerHeight) continue;
        const hit = document.elementFromPoint(x, y);
        if (hit && group.contains(hit)) return { x, y };
      }
      return null;
    }, edgeSelector);
    assert(point, "The selected story edge must expose a real hit point in the graph viewport");
    await browser.performActions([{ type: "pointer", id: "story-graph-edge-pointer", parameters: { pointerType: "mouse" }, actions: [
      { type: "pointerMove", origin: "viewport", x: point.x, y: point.y },
      { type: "pointerDown", button: 0 }, { type: "pointerUp", button: 0 },
    ] }]);
    await browser.releaseActions();
    await browser.waitUntil(async () => await $(".a4n-story-graph-edge-direction").isDisplayed());
  }, expected: ["graph-edge-1", "source-node", "target-node"], after: async () => {
    const direction = await browser.execute(() => {
      const row = document.querySelector(".a4n-story-graph-edge-direction");
      return {
        from: row?.querySelector(".a4n-story-graph-edge-from")?.textContent.trim(),
        to: row?.querySelector(".a4n-story-graph-edge-to")?.textContent.trim(),
        icon: row?.querySelector("svg.lucide-arrow-right")?.getAttribute("aria-hidden"),
        spokenDirection: row?.querySelector(".sr-only")?.textContent.trim(),
      };
    });
    assert.deepEqual(direction, { from: "source-node", to: "target-node", icon: "true", spokenDirection: "到" });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("save_galgame_story_graph")), false);
    return { ...direction, selectedWithoutMutation: true };
  }, note: "只选中并读取图边详情；起点/终点方向用 Lucide 表示，读屏仍读到方向词。" },

  { id: "MX01", title: "画布保存视角", task: "按已保存视角阅读节点与主要数据流", open: () => openCanvas(), widths: canvasWidths, count: [".react-flow__node", 17], expected: ["节点"], note: "图中17节点均已加载；初始呈现遵守保存视角，不声称所有节点同时在屏内。" },
  ...[
    ["MX02", 6001, "文本输入", "文本"], ["MX03", 6013, "LLM节点", "指令"],
    ["MX04", 6003, "图片生成节点", "图片配置"], ["MX05", 6005, "视频生成节点", "输入模式"],
    ["MX06", 6017, "缺失首尾帧节点", "首尾帧"], ["MX07", 6008, "组装顺序", "成片轨道"],
  ].map(([id, node, title, text]) => ({ id, title, task: `选择${title}并修改前核对属性`, open: () => openCanvas(node), widths: canvasWidths, expected: [text], note: "1024px属性必须有入口；参数、输入与输出分组和字号待复核。" })),
  { id: "MX08", title: "画布工作流", task: "读取已完成节点、失败原因及恢复入口", open: () => openCanvas(6008), widths: canvasWidths, expected: ["结尾镜头缺少首帧与尾帧", "运行状态", "失败节点", "定位节点"], after: async (record) => {
    const workflow = 'section[aria-label="工作流运行记录"]';
    const rows = await $$(`${workflow} > div[class*="mt-2"][class*="divide-y"] > div`);
    assert.equal(rows.length, 3, "The populated run timeline must keep all three fixture events");
    const lastEvent = rows.at(-1);
    const reachability = await reachByWheel(lastEvent);
    assert.equal(reachability.reached, true, "The oldest visible run event must be reachable inside the inspector scroll area");
    const text = await lastEvent.getText();
    assert(text.includes("47%") && text.includes("首帧和连续镜头已完成"));
    record.reachability = reachability;
    record.visualReview = `主流程已看${record.width}px末态，无遮挡或可确认发现；非独立审图。`;
    return { eventCount: rows.length, lastEvent: text };
  }, note: "运行状态位于属性尾部；滚到最早事件确认长面板中的历史进度仍能完整阅读。" },
  ...[
    ["MX09", 6014, "文本核验", "text"], ["MX10", 6016, "尾帧核验", "image"],
    ["MX11", 6009, "成片输出", "video"], ["MX12", 6011, "雨声监听", "audio"],
  ].map(([id, node, title, kind]) => ({
    id, title: `画布${kind}内容与属性`, task: `打开${title}的已有内容并返回节点属性`,
    open: () => openCanvasContent(node, title), widths: canvasWidths, scope: dialog,
    expected: [title], images: kind === "image" ? 1 : 0,
    after: async (record, width) => {
      const playback = kind === "video" || kind === "audio" ? await provePlayback(`${dialog} ${kind}`) : null;
      await browser.keys("Escape");
      await $(`//div[@role="dialog" and .//h3[normalize-space()="${title} 内容"]]`).waitForExist({ reverse: true });
      const scope = await canvasInspector();
      await ready();
      const titleInput = await $(scope).$('input[type="text"]');
      assert.equal((await reachByWheel(titleInput)).reached, true);
      assert.equal(await titleInput.getValue(), title);
      if (kind !== "video") {
        const mode = fixtures.byProject[6].get_canvas_graph.nodes.find((item) => item.id === node).config_json.display_mode;
        assert.equal(await $(scope).$('button[role="combobox"]').getText(), mode === "detail" ? "详细" : "摘要");
      }
      record.properties = await capture({ id, scope }, width, "-properties");
      assert.equal(record.properties.overflow, 0);
      assert.deepEqual(record.properties.unnamedFields, []);
      return { playback, nodeId: node, title, propertiesRead: true };
    },
    note: "使用真实内容与媒体解码，另验属性本体及返回路径。",
  })),
  { id: "MX13", title: "画布键盘添加", task: "用键盘从节点库添加文本输入", open: () => openCanvas(), widths: [1440], expected: ["节点库"], after: async () => { const before = (await $$(".react-flow__node")).length; const template = await $('button[draggable="true"]'); await template.execute(element => element.focus()); await browser.keys("Enter"); await browser.waitUntil(async () => (await $$(".react-flow__node")).length === before + 1, { timeout: 2500, timeoutMsg: "Desktop keyboard activation must add a node" }); return { before, after: before + 1 }; }, note: "保留拖动，同时要求按钮有键盘激活行为。" },
  { id: "MX14", title: "画布移动节点库", task: "在窄屏找到节点库并查看全部类别", open: async () => { await openCanvas(); await $('button[aria-label="打开节点库"]').click(); await $(dialog).waitForDisplayed(); }, widths: [390], scope: dialog, expected: ["节点库", "文本输入", "视频输出"], after: async () => {
    const templates = await $(dialog).$$('button[draggable]');
    assert.equal(templates.length, 19);
    const last = templates.at(-1);
    assert.equal((await reachByWheel(last)).reached, true, "The final node template must be reachable through the library scroll container");
    assert.equal(await last.getText(), "音频输出");
    return { templates: templates.length, lastTemplate: "音频输出", lastTemplateReachable: true };
  }, note: "19种节点完整存在；真实滚到末项音频输出，不以DOM文字包含代替末项可达。" },
  { id: "MX15", title: "画布请求预览", task: "从所选视频节点查看模型请求", open: async () => { await openCanvas(6005); if ((await browser.execute(() => innerWidth)) <= 1100) await browser.keys("Escape"); await openMenu(".a4n-page-header", "查看模型请求"); await $(dialog).waitForDisplayed(); }, widths: canvasWidths, scope: dialog, expected: ["请求"], note: "预览与运行分开，已保存图才允许请求检查。" },
  { id: "MX16", title: "画布裁剪属性", task: "核对裁剪起止时间并修改本地结束时间", open: () => openCanvas(6007), widths: canvasWidths, expected: ["开始秒", "结束秒"], after: async () => {
    const fields = await $(await canvasInspector()).$$('input[inputmode="decimal"]');
    assert.equal(fields.length, 2);
    assert.equal(Number(await fields[0].getValue()), 0);
    assert.equal(Number(await fields[1].getValue()), 6);
    assert.equal((await reachByWheel(fields[1])).reached, true);
    await fields[1].setValue("5.25");
    assert.equal(await fields[1].getValue(), "5.25");
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("save_canvas_graph")), false);
    return { nodeId: 6007, trimStart: 0, originalEnd: 6, draftEnd: 5.25, saved: false };
  }, note: "裁剪属性独立验收，末项可达且真实接受本地输入；不执行裁剪或保存。" },
  { id: "MX17", title: "画布指定时间抽帧", task: "从尾帧切换为指定时间并输入时间点", open: async () => {
    await openCanvas(6015);
    const scope = await canvasInspector();
    const kind = await $(scope).$('button[role="combobox"]');
    assert.equal(await kind.getText(), "尾帧");
    await selectOption(kind, "指定时间");
    const seconds = await $(scope).$('input[inputmode="decimal"]');
    assert.equal((await reachByWheel(seconds)).reached, true);
    await seconds.setValue("2.5");
  }, widths: canvasWidths, expected: ["指定时间", "时间秒"], after: async () => {
    assert.equal(await $(await canvasInspector()).$('input[inputmode="decimal"]').getValue(), "2.5");
    assert.equal(await browser.execute(() => window.__uiAudit.commands.some(command => ["save_canvas_graph", "run_canvas_node"].includes(command))), false);
    return { nodeId: 6015, specifiedFrameSeconds: 2.5, saved: false };
  }, note: "验证尾帧到指定时间的实际切换，字段可达；不运行抽帧或保存。" },
  { id: "MX18", title: "画布已有视频引用", task: "展开当前视频输入并核对资源和缩略图", open: async () => { await openCanvas(6006); await openDetails(`${await canvasInspector()} details`); }, widths: canvasWidths, expected: ["视频资源编号"], after: async () => {
    const scope = await canvasInspector();
    const field = await $(scope).$('input[inputmode="numeric"]');
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6006);
    assert.equal(Number(await field.getValue()), source.ref_id);
    assert.equal((await reachByWheel(field)).reached, true);
    await field.click();
    const image = await $(scope).$('img');
    assert.equal(await image.execute(element => element.currentSrc), source.config_json.thumbnail_path);
    assert.equal(await image.execute(element => element.complete && element.naturalWidth > 0), true);
    return { nodeId: 6006, videoId: source.ref_id, thumbnailMatches: true };
  }, note: "当前编号 34001 与当前真实缩略图必须同时匹配，不以参数标题代替内容身份。" },
  { id: "MX19", title: "画布连线属性", task: "点击真实连线并核对两端节点与端口", open: openCanvasEdge, widths: canvasWidths, expected: ["从「雨夜的旧书店」传递到「镜头描述模板」", "来源端口", "目标端口", "连接类型"], after: async () => {
    const details = await $(await canvasInspector()).$('details');
    const values = await details.$('div').$$(':scope > span');
    const source = fixtures.byProject[6].get_canvas_graph.edges.find(edge => edge.id === 6801);
    assert.equal(values.length, 8);
    assert.deepEqual(await Promise.all([values[1].getText(), values[3].getText(), values[5].getText()]), [source.role, source.source_port, source.target_port]);
    assert.equal((await reachByWheel(values.at(-1))).reached, true);
    return { edgeId: source.id, sourcePort: source.source_port, targetPort: source.target_port, role: source.role };
  }, note: "SVG 实际点击后逐值核对 context/text/input，并检查末项可达。" },
  { id: "MX20", title: "画布单节点运行确认", task: "核对当前运行目标与输入来源规则后取消", open: async () => { await openCanvas(6005); if ((await browser.execute(() => innerWidth)) <= 1100) await browser.keys("Escape"); await openMenu(".a4n-page-header", "只运行所选节点"); await $(dialog).waitForDisplayed(); }, widths: [1440, 1024, 900, 390], scope: dialog, expected: ["只运行此节点", "连续镜头", "最近成功结果", "无结果时先运行到此节点", "取消"], after: async () => {
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    await $(dialog).waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("run_canvas_node")), false);
    assert((await $('.react-flow__node[data-id="6005"]').getAttribute("class")).includes("selected"));
    return { selectedNodeId: 6005, cancelledBeforeCanvasRun: true };
  }, note: "说明复用的上游结果与无结果时的处理；取消后保留节点选择，不启动执行。" },
  { id: "MX21", title: "画布图片主体选择", task: "点击已有视频的缩略图主体选择对应节点", open: async () => {
    await openCanvasThumbnail();
    const node = await $('.react-flow__node[data-id="6006"]');
    assert.equal((await node.getAttribute("class")).includes("selected"), false);
    await node.$('img').click();
    await browser.waitUntil(async () => (await node.getAttribute("class")).includes("selected"), {
      timeout: 2500, timeoutMsg: "The canvas thumbnail body must select its node",
    });
    assert.equal(await $(dialog).isExisting(), false, "The thumbnail body must retain the canvas selection task");
  }, widths: canvasWidths, expected: ["已有过场"], images: 1, note: "真实点击当前缩略图，核对选中 6006；独立放大使用另外的按钮。" },
  { id: "MX22", title: "画布图片独立预览", task: "独立放大当前缩略图并用Escape返回原按钮", open: openCanvasThumbnail, widths: canvasWidths, expected: ["已有过场"], images: 1, after: async (record, width) => {
    const selected = () => browser.execute(() => [...document.querySelectorAll('.react-flow__node.selected')].map(node => node.dataset.id));
    const before = await selected();
    assert.deepEqual(before, []);
    const button = await $('.react-flow__node[data-id="6006"] button[aria-label="图片预览"]');
    let mobileTarget = null;
    if (width <= 640) {
      const bounds = await button.execute(element => {
        const buttonRect = element.getBoundingClientRect();
        const canvasRect = document.querySelector(".react-flow").getBoundingClientRect();
        const zoom = new DOMMatrixReadOnly(getComputedStyle(document.querySelector(".react-flow__viewport")).transform).a;
        const hitTarget = getComputedStyle(element, "::before");
        const targetWidth = parseFloat(hitTarget.width) * zoom;
        const hitPoint = document.elementFromPoint(
          buttonRect.left + (buttonRect.width + targetWidth) / 2,
          buttonRect.top + buttonRect.height / 2,
        );
        return {
          button: buttonRect.toJSON(),
          canvas: canvasRect.toJSON(),
          targetWidth,
          targetHeight: parseFloat(hitTarget.height) * zoom,
          expandedHit: hitPoint?.closest('button[aria-label="图片预览"]') === element,
        };
      });
      assert(bounds.button.left >= bounds.canvas.left && bounds.button.right <= bounds.canvas.right,
        "The mobile image preview target must stay inside the visible canvas");
      assert(bounds.targetWidth >= 44 && bounds.targetHeight >= 44,
        `The scaled mobile preview target must stay at least 44 CSS px: ${JSON.stringify(bounds)}`);
      if (bounds.targetWidth > bounds.button.width + 1) {
        assert.equal(bounds.expandedHit, true, "The transparent area beyond the icon button must still activate its preview");
      }
      mobileTarget = {
        buttonWidth: bounds.button.width,
        buttonHeight: bounds.button.height,
        targetWidth: bounds.targetWidth,
        targetHeight: bounds.targetHeight,
        expandedHit: bounds.expandedHit,
      };
    }
    await button.waitForClickable();
    await button.click();
    await $(dialog).waitForDisplayed();
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6006).config_json.thumbnail_path;
    assert.equal(await $(`${dialog} .a4n-image-lightbox-image`).execute(element => element.currentSrc), source);
    record.lightbox = await capture({ id: record.id, scope: dialog }, width, "-lightbox");
    assert.equal(record.lightbox.overflow, 0);
    assert.equal(record.lightbox.images.filter(image => image.loaded && image.src === source).length, 1);
    await browser.keys("Escape");
    await $(dialog).waitForExist({ reverse: true });
    await browser.waitUntil(async () => button.execute(element => element === document.activeElement), {
      timeout: 2500, timeoutMsg: "Closing the canvas image preview must restore focus to its own preview button",
    });
    assert.deepEqual(await selected(), before, "Independent preview must preserve the canvas selection");
    return { nodeId: 6006, mobileTarget, imageMatches: true, selectionUnchanged: true, focusRestored: true };
  }, note: "独立记录灯箱图；Escape 后精确核对原预览按钮获得焦点，不仅检查弹窗消失。" },
  { id: "MX23", title: "画布缩放与交互锁", task: "放大、缩小、适配画布并锁定后恢复编辑", open: () => openCanvas(), widths: canvasWidths, expected: ["影像画布"], after: async (record, width) => {
    const controls = '.a4n-flow-controls';
    const expectedControls = width < 640 ? 4 : 5;
    assert.equal((await $$(`${controls} button`)).length, expectedControls);
    assert.equal((await $$(`${controls} button > svg.lucide`)).length, expectedControls, "All canvas controls must use Lucide icons");
    const viewport = () => browser.execute(() => {
      const canvas = document.querySelector('.react-flow').getBoundingClientRect();
      const matrix = new DOMMatrixReadOnly(getComputedStyle(document.querySelector('.react-flow__viewport')).transform);
      const nodes = [...document.querySelectorAll('.react-flow__node')];
      return {
        x: matrix.e, y: matrix.f, zoom: matrix.a, nodes: nodes.length,
        nodesInView: nodes.filter(node => {
          const box = node.getBoundingClientRect();
          return box.left >= canvas.left - 2 && box.right <= canvas.right + 2 && box.top >= canvas.top - 2 && box.bottom <= canvas.bottom + 2;
        }).length,
        draggable: nodes.filter(node => node.classList.contains('draggable')).length,
      };
    });
    const before = await viewport();
    assert.equal(before.nodes, 17);
    assert(before.draggable > 0, "The initial canvas must allow editing");
    await $(`${controls} .react-flow__controls-zoomin`).click();
    await browser.waitUntil(async () => (await viewport()).zoom > before.zoom + 0.001);
    const zoomedIn = await viewport();
    await $(`${controls} .react-flow__controls-zoomout`).click();
    await browser.waitUntil(async () => (await viewport()).zoom < zoomedIn.zoom - 0.001);
    const zoomedOut = await viewport();
    await $(`${controls} .react-flow__controls-fitview`).click();
    await browser.waitUntil(async () => {
      const next = await viewport();
      return Math.abs(next.zoom - zoomedOut.zoom) > 0.001 || Math.abs(next.x - zoomedOut.x) > 1 || Math.abs(next.y - zoomedOut.y) > 1;
    }, { timeout: 2500, timeoutMsg: "Fit View must change the actual saved viewport" });
    const fitted = await viewport();
    assert(fitted.nodesInView >= before.nodesInView, "Fit View must not reduce the number of fully visible nodes");
    const lock = await $(`${controls} .react-flow__controls-interactive`);
    assert.equal(await lock.getAttribute('aria-pressed'), 'false');
    await lock.click();
    await browser.waitUntil(async () => (await lock.getAttribute('aria-pressed')) === 'true' && (await viewport()).draggable === 0);
    assert.equal(await lock.$('svg.lucide-lock-keyhole').isExisting(), true);
    record.locked = await capture({ id: record.id }, width, "-locked");
    await lock.click();
    await browser.waitUntil(async () => (await lock.getAttribute('aria-pressed')) === 'false' && (await viewport()).draggable === before.draggable);
    assert.equal(await lock.$('svg.lucide-lock-keyhole-open').isExisting(), true);
    return { before, zoomedIn, zoomedOut, fitted, restoredDraggable: (await viewport()).draggable };
  }, note: "检查真实 viewport 与交互状态；fit 遵守现有最小缩放，不声称窄屏能同时容纳全部节点。结束时恢复可编辑。" },
  { id: "MX24", title: "画布文本模板属性", task: "核对6002的完整模板并修改本地文本类型", frequency: "中", open: () => openCanvas(6002), widths: canvasWidths, expected: ["模板", "文本类型"], after: async () => {
    const scope = await canvasInspector();
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6002);
    const template = await $(scope).$("textarea");
    assert.equal(await template.getValue(), source.config_json.template);
    const edited = `${source.config_json.template}\n保持最后一镜的信封朝向。`;
    await template.setValue(edited);
    await selectOption(await $(scope).$('button[role="combobox"]'), "视频文本");
    await focusTextEnd(`${scope} textarea`);
    assert.equal(await template.getValue(), edited);
    assert.equal(await $(scope).$('button[role="combobox"]').getText(), "视频文本");
    assert.equal(await browser.execute(() => window.__uiAudit.commands.some(command => ["save_canvas_graph", "run_canvas_node"].includes(command))), false);
    return { nodeId: 6002, originalTemplateMatched: true, fullDraftMatched: true, outputKind: "video_prompt", saved: false };
  }, note: "原17节点不变；模板全文与{{input}}保留，只产生本地草稿，不生成新的运行产物。" },
  { id: "MX25", title: "画布图片输出属性", task: "核对6004当前成图、上游输入和保存资源身份", open: () => openCanvas(6004), widths: canvasWidths, expected: ["书店首帧", "输入", "输出", "查看内容"], images: 1, after: async () => {
    const scope = await canvasInspector();
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6004);
    const image = await $(scope).$("img");
    assert.equal(await image.execute(element => element.currentSrc), source.config_json.file_path);
    assert.equal(await image.execute(element => element.complete && element.naturalWidth > 0), true);
    await openDetails(`${scope} details`);
    const details = await $(scope).$("details");
    assert((await details.getText()).includes(`generated_image #${source.ref_id}`));
    assert((await details.getText()).includes(source.config_json.file_path));
    assert.equal((await reachByWheel(details)).reached, true);
    return { nodeId: 6004, imageId: source.ref_id, imageMatches: true, resourcePathMatches: true };
  }, note: "使用6004既有60001成图，不把旁边图片探针或缩略图当成这个输出节点的证据。" },
  { id: "MX26", title: "画布音频输入属性", task: "核对6010的63001音频引用并播放真实3秒样本", open: async () => { await openCanvas(6010); await openDetails(`${await canvasInspector()} details`); }, widths: canvasWidths, expected: ["雨声环境", "音频资源编号"], after: async (record, width) => {
    const scope = await canvasInspector();
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6010);
    const reference = await $(scope).$('input[inputmode="numeric"]');
    assert.equal(Number(await reference.getValue()), source.ref_id);
    assert.equal((await reachByWheel(reference)).reached, true);
    await $(scope).$('.//button[normalize-space()="查看内容"]').click();
    await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "雨声环境 内容");
    const audio = `${dialog} audio`;
    await waitForPlayable(audio);
    assert.equal(await $(audio).execute(element => element.currentSrc), source.config_json.file_path);
    const playback = await provePlayback(audio);
    assert(Math.abs(playback.duration - source.config_json.duration_secs) < 0.05);
    assert.equal(source.config_json.duration_secs, 3);
    record.content = await capture({ id: record.id, scope: dialog }, width, "-content");
    return { nodeId: 6010, mediaAssetId: source.ref_id, declaredDuration: 3, ...playback };
  }, note: "沿用实际3秒WAV、精确核对引用与解码时长，不改输入时长或伪造媒体结果。" },
  { id: "MX27", title: "画布音频输出属性", task: "核对6012的上游雨声监听和已保存音频路径", open: () => openCanvas(6012), widths: canvasWidths, expected: ["环境声输出", "雨声监听", "输出", "查看内容"], after: async () => {
    const scope = await canvasInspector();
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6012);
    await openDetails(`${scope} details`);
    const details = await $(scope).$("details");
    assert((await details.getText()).includes(String(source.config_json.media_asset_id)));
    assert((await details.getText()).includes(source.config_json.file_path));
    assert.equal((await reachByWheel(details)).reached, true);
    return { nodeId: 6012, mediaAssetId: source.config_json.media_asset_id, resourcePathMatches: true };
  }, note: "此输出本来没有可编辑参数；保留输入连接与已保存结果，不为审查添加假参数或时长字段。" },
  { id: "MX28", title: "画布图片输入草稿", task: "从节点库新建image_input并核对空资源引用", frequency: "中", open: async () => { await openCanvasDraft("图片输入"); await openDetails(`${await canvasInspector()} details`); }, widths: canvasWidths, expected: ["图片输入", "图片资源编号"], after: async () => {
    const scope = await canvasInspector();
    const field = await $(scope).$('input[inputmode="numeric"]');
    assert.equal(await field.getValue(), "");
    assert.equal((await reachByWheel(field)).reached, true);
    await field.click();
    assert.doesNotMatch(await $(scope).getText(), /已关联|已绑定/, "An empty resource ID must not be reported as linked");
    assert.doesNotMatch(await $(".react-flow__node.selected").getText(), /已绑定/, "The canvas card must agree with its missing-reference warning");
    return inspectCanvasDraft("ImageInputNode");
  }, note: "真实节点库创建第18个本地节点，原17节点样本保留；空引用和未运行状态不伪装成导入成功。" },
  { id: "MX29", title: "画布视频探针草稿", task: "从节点库新建video_probe并编辑标题和显示模式", frequency: "中", open: () => openCanvasDraft("视频探针"), widths: canvasWidths, expected: ["标题", "展示模式", "尚未观察到运行结果"], after: async () => {
    const scope = await canvasInspector();
    const title = await $(scope).$('input[type="text"]');
    assert.equal(await title.getValue(), "");
    await title.setValue("中间视频核验");
    const mode = await $(scope).$('button[role="combobox"]');
    await selectOption(mode, "详细");
    assert.equal(await title.getValue(), "中间视频核验");
    assert.equal(await mode.getText(), "详细");
    assert((await $(scope).getText()).includes("尚未观察到运行结果"));
    return inspectCanvasDraft("VideoProbeNode");
  }, note: "真实修改探针草稿，保留尚未运行的说明，不注入视频、缩略图或观察数据。" },
  { id: "MX30", title: "画布文本输出草稿", task: "从节点库新建text_output并核对终点输入与未运行结果", frequency: "中", open: () => openCanvasDraft("文本输出"), widths: canvasWidths, expected: ["文本输出", "输入", "输出", "运行后在这里显示最终交付结果"], after: () => inspectCanvasDraft("TextOutputNode"), note: "实际输出节点没有额外参数；检查其真实属性与未运行提示，不复用已有文本探针充当输出。" },
  { id: "MX31", title: "独立画布返回", task: "从独立画布通过页头返回项目列表", open: () => openCanvas(), widths: [1440, 390], expected: ["山海之间"], after: async () => {
    await $('button[aria-label="返回上一页"]').click();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/");
    await $(".a4n-home-content").waitForDisplayed();
    return { returnedToProjectList: true, path: "/" };
  }, note: "画布是独立 project_kind=canvas；核对其真实页头返回合同，不将视频工作区映射到未注册的 /studio 路由。" },
  { id: "MX32", title: "画布视口保存", task: "用画布缩放控件调整视口并保存当前图", open: () => openCanvas(), widths: [1440, 390], expected: ["影像画布"], after: async (_record, width) => {
    if (width >= 640) {
      assert.equal(await $(".a4n-studio-minimap").isDisplayed(), false, "The minimap starts collapsed so it cannot cover nodes");
      await $(".react-flow__controls-minimap").click();
      await $(".a4n-studio-minimap").waitForDisplayed();
      await $(".react-flow__controls-minimap").click();
      await browser.waitUntil(async () => !(await $(".a4n-studio-minimap").isDisplayed()));
    }
    const before = await browser.execute(() => {
      const matrix = new DOMMatrixReadOnly(getComputedStyle(document.querySelector(".react-flow__viewport")).transform);
      return { x: matrix.e, y: matrix.f, zoom: matrix.a };
    });
    await $(".react-flow__controls-zoomin").click();
    await buttonText("保存画布").waitForDisplayed();
    await browser.saveScreenshot(`${output}/${phase}-B84-video-canvas-zoom-${width}.png`);
    await buttonText("保存画布").click();
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.requests.some(item => item.command === "save_canvas_graph")));
    const saved = await browser.execute(() => window.__uiAudit.requests.findLast(item => item.command === "save_canvas_graph").body.input.graph.viewport_json);
    assert.equal(Number.isFinite(saved.x) && Number.isFinite(saved.y) && Number.isFinite(saved.zoom), true);
    assert.notEqual(saved.zoom, before.zoom, "The write must carry the user's changed viewport");
    return { before, savedViewport: saved };
  }, note: "检查用户视口进入真实画布写请求；不只检查保存 Toast。" },
  { id: "MX33", title: "画布节点拖拽预览图标", task: "从节点库拖动节点并核对预览沿用 Lucide 图标", open: () => openCanvas(), widths: [1440], expected: ["节点库"], after: async (_record, width) => {
    const before = (await $$(".react-flow__node")).length;
    const template = await $('button[draggable="true"]');
    const drag = await template.execute(element => {
      const sourceIcon = element.querySelector("svg.lucide");
      let preview = null;
      let anchor = null;
      const dataTransfer = new DataTransfer();
      Object.defineProperty(dataTransfer, "setDragImage", {
        configurable: true,
        value(image, x, y) {
          preview = image.cloneNode(true);
          preview.dataset.auditDragPreview = "true";
          Object.assign(preview.style, { left: "330px", top: "120px", zIndex: "2147483647" });
          document.body.appendChild(preview);
          anchor = { x, y };
        },
      });
      const event = new DragEvent("dragstart", { bubbles: true, cancelable: true, dataTransfer });
      element.dispatchEvent(event);
      const previewIcon = preview?.querySelector(".a4n-studio-node-icon svg.lucide");
      return {
        prevented: event.defaultPrevented,
        dragTypes: [...dataTransfer.types],
        dragLabel: dataTransfer.getData("application/a4n-studio-node-label"),
        sourceLabel: element.textContent.trim(),
        sourceIconClass: sourceIcon?.getAttribute("class") ?? null,
        previewIconClass: previewIcon?.getAttribute("class") ?? null,
        previewIconWidth: previewIcon?.getAttribute("width") ?? null,
        iconText: preview?.querySelector(".a4n-studio-node-icon")?.textContent?.trim() ?? null,
        anchor,
      };
    });
    try {
      assert.equal(drag.prevented, false);
      assert(drag.dragTypes.includes("application/a4n-studio-node-kind"), `The drag payload must retain its node type: ${JSON.stringify(drag)}`);
      assert.equal(drag.dragLabel, drag.sourceLabel, "The drag payload must retain the node label");
      assert(drag.sourceIconClass?.includes("lucide"), "The node library entry must have a Lucide icon");
      assert(drag.previewIconClass, `The real dragstart handler must install its drag image: ${JSON.stringify(drag)}`);
      assert.equal(drag.previewIconClass, drag.sourceIconClass, "The drag image must reuse the source Lucide icon");
      assert.equal(drag.iconText, "", "The drag image must not replace its icon with a character glyph");
      assert(drag.anchor && drag.anchor.x > 0 && drag.anchor.y > 0, "The drag image must retain its centered pointer anchor");
      await browser.saveScreenshot(`${output}/${phase}-MX33-drag-preview-${width}.png`);
      assert.equal((await $$(".react-flow__node")).length, before, "Starting a drag must not create a node before drop");
      return { ...drag, nodeCountUnchanged: true, screenshot: `${phase}-MX33-drag-preview-${width}.png` };
    } finally {
      await browser.execute(() => document.querySelector('[data-audit-drag-preview="true"]')?.remove());
    }
  }, note: "只触发拖拽开始并截图真实 setDragImage 节点；不执行 drop 或新增节点。" },
  ...[["MS01", 2, "漫画"], ["MS02", 3, "视频"], ["MS03", 5, "Galgame"]].map(([id, project, label]) => ({ id, title: `${label}来源同步冲突`, task: "比较三条来源变化与当前改编，关闭前不应用决定", frequency: "低", open: () => openSourceSync(project), scope: dialog, count: [`${dialog} article`, 3], expected: ["冲突清单", "来源变化", "当前项目", "未寄出的信"], after: inspectSourceSyncEnd, note: "typed dry_run预览；真实滚到末条冲突并补充conflicts-end截图，不为应用/解绑提供假成功mutation。" })),
];

describe("Populated media pages and task dialogs", () => {
  before(async () => {
    await mkdir(output, { recursive: true });
    fixtures = await buildPageFixtures(browser.options.baseUrl);
    const canvasProject = fixtures.projectRecords.find((project) => project.id === 6);
    assert.ok(canvasProject, "Media canvas fixture requires project 6");
    canvasProject.project_kind = "canvas";
    canvasProject.project_type = "canvas";
    fixtures.list_projects.items = fixtures.projectRecords;
    fixtures.byProject[6].list_canvas_text_assets = [];
    const canvas = fixtures.byProject[6].get_canvas_graph;
    canvas.graph.graph_kind = "video_studio";
    fixtures.byProject[3].get_canvas_graph = {
      ...canvas,
      graph: { ...canvas.graph, id: 3, graph_kind: "video_studio", name: "来源视频画布" },
      nodes: [],
      edges: [],
    };
    fixtures.save_canvas_graph = {
      graph: { ...canvas.graph, version: canvas.graph.version + 1 },
      nodes: canvas.nodes,
      edges: canvas.edges,
      node_ids: Object.fromEntries(canvas.nodes.map((node) => [String(node.id), node.id])),
    };
    await installUiFixtures(browser, fixtures);
    await browser.url("/");
    await browser.switchToWindow(await browser.getWindowHandle());
    await $(".a4n-home-content").waitForDisplayed();
    await seedLocalModels(browser);
  });

  after(async () => {
    await writeFile(`${output}/${phase}-report.json`, JSON.stringify(report, null, 2));
    const rows = report.map(item => `| ${item.id} | ${item.width} | ${item.title} | ${item.functional} | ${item.overflow ?? "未测"} | ${item.visualReview} | ${item.file ?? "无截图"} |`);
    await writeFile(`${output}/${phase}-ledger.md`, [
      "# 媒体页面逐项审查台账", "", "功能断言与视觉复审分开记录；断言通过不等于页面设计通过。详细任务、已知缺点、保留要求、控制尺寸和请求轨迹见同名 JSON。", "",
      "| ID | 宽度 | 视图 | 功能断言 | 横向溢出 | 视觉复审 | 截图 |", "| --- | --- | --- | --- | --- | --- | --- |", ...rows, "",
    ].join("\n"));
  });

  afterEach(async function () {
    const record = this.currentTest.ctx.mediaRecord;
    if (!record) return;
    record.functional = this.currentTest.state;
    if (this.currentTest.err) record.failure = this.currentTest.err.message;
    if (this.currentTest.state === "failed") {
      const failed = await capture({ id: record.id }, record.width, "-failure");
      record.failureState = failed;
      if (!record.file) Object.assign(record, failed);
    }
    report.push(record);
    await writeFile(`${output}/${phase}-${record.id}-${record.width}.json`, JSON.stringify(record, null, 2));
    if (record.id === "MG14") await installUiFixtures(browser, fixtures);
  });

  for (const entry of cases) for (const width of entry.widths ?? widths) {
    it(`${entry.id}-${width} ${entry.title}`, async function () {
      const record = this.mediaRecord = {
        id: entry.id, title: entry.title, width, task: entry.task, frequency: entry.frequency ?? "高",
        knownConcern: entry.note, retained: "原暖书房与iOS视觉、既有功能、错误反馈、键盘访问",
        visualReview: "待独立实屏复核", functional: "running",
      };
      try {
        await browser.setWindowSize(width, 1000);
        await entry.open();
        if (entry.tail) record.reachability = await reachByWheel(entry.tail, entry.axis);
        await ready();
        Object.assign(record, await capture(entry, width));
        assert.equal(record.width, width);
        assert.equal(record.overflow, 0, `${entry.id}: document horizontal overflow`);
        assert.deepEqual(record.unexpected, [], "Missing fixtures are failures, not empty states");
        const readableContent = [record.text, ...record.controls.map(control => control.value ?? "")].join("\n");
        for (const expected of entry.expected ?? []) assert(readableContent.includes(expected), `${entry.id}: missing populated content ${expected}`);
        if (entry.count) assert.equal((await $$(entry.count[0])).length, entry.count[1], `${entry.id}: populated record count`);
        if (entry.images) assert(record.images.filter(image => image.loaded).length >= entry.images, `${entry.id}: no decoded visible image`);
        if (entry.tail) assert.equal(record.reachability.reached, true, `${entry.id}: last task control cannot be reached with real scrolling`);
        if (entry.after) {
          record.interaction = await entry.after(record, width);
          record.afterInteraction = await capture({ id: entry.id }, width, "-interaction");
          assert.deepEqual(record.afterInteraction.unexpected, []);
        }
        assert.deepEqual(record.errors, [], "Browser errors must remain visible");
        if (record.afterInteraction) assert.deepEqual(record.afterInteraction.errors, [], "Browser errors must remain visible after the interaction");
      } catch (error) {
        record.failure = error instanceof Error ? error.message : String(error);
        throw error;
      }
    });
  }

  it("UX-B60 ignores an active Manga stage click while keeping its page draft", async function () {
    this.mediaRecord = { id: "UX-B60-Manga", title: "漫画当前阶段点击", width: 1440, task: "当前阶段不应触发放弃提示" };
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.execute(() => localStorage.setItem("a4n:comic:2:stage", "prompts"));
    await openRoute("/comic-project/2");
    const chapterButtons = await $('nav[aria-label="漫画章节"]').$$("ul > li > button");
    await chapterButtons[0].click();
    await $('nav[aria-label="漫画页列表"]').waitForDisplayed();
    await selectComicPage(3);
    await openMenu(comicPrompt, "编辑提示词");
    const prompt = await $(`${comicPrompt} textarea`);
    const original = await prompt.getValue();
    const draft = `${original}\n当前漫画页保留的草稿。`;
    await prompt.setValue(draft);
    const activeStage = await $('button[id="comic-stages-2-tab-prompts"]');
    assert.equal(await activeStage.getAttribute("aria-selected"), "true");
    await activeStage.click();
    assert.equal(await $$('[role="dialog"]').length, 0, "Clicking the active Manga stage must not prompt for discard");
    assert.equal(await prompt.getValue(), draft);
    await $(comicPrompt).$('.//button[normalize-space()="取消"]').click();
    this.mediaRecord.interaction = { activeStageClickPreservedPromptDraft: true, explicitCancelCleanedDraft: true };
  });

  it("UX-B60 protects Galgame drafts only when the user changes stage", async function () {
    const record = this.mediaRecord = { id: "UX-B60", title: "Galgame阶段草稿边界", width: 1440, task: "验证阶段自点击无副作用，并在真实阶段切换前保护输入" };
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.execute(() => localStorage.setItem("a4n:galgame:5:stage", "script"));
    await openGalgamePerformance(6);

    const scriptStage = await $('button[id="galgame-stages-5-tab-script"]');
    assert.equal(await scriptStage.getAttribute("aria-selected"), "true");
    const cgPrompt = await $('section[aria-label="编辑"] textarea[placeholder="事件 CG 画面描述"]');
    const originalCgPrompt = await cgPrompt.getValue();
    const cgDraft = `${originalCgPrompt}\n当前舞台内保留的草稿。`;
    await cgPrompt.setValue(cgDraft);
    await scriptStage.click();
    await browser.waitUntil(async () => (await cgPrompt.getValue()) === cgDraft);
    assert.equal(await $$('[role="dialog"]').length, 0, "Clicking the active stage must not ask to discard its draft");

    const adaptationStage = await $('button[id="galgame-stages-5-tab-adaptation"]');
    await adaptationStage.click();
    await $(dialog).waitForDisplayed();
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    assert.equal(await scriptStage.getAttribute("aria-selected"), "true");
    assert.equal(await cgPrompt.getValue(), cgDraft, "Cancel must preserve the Galgame CG draft");
    await adaptationStage.click();
    await $(dialog).waitForDisplayed();
    await $(dialog).$('.//button[normalize-space()="放弃草稿"]').click();
    await browser.waitUntil(async () => (await adaptationStage.getAttribute("aria-selected")) === "true");
    await $('//summary[contains(normalize-space(.), "改编要求与参考")]').click();
    const instruction = await $('//label[.//span[normalize-space()="本次改编要求"]]/textarea');
    await instruction.waitForDisplayed();
    const instructionDraft = `${await instruction.getValue()}\n只用于这次改编的要求。`;
    await instruction.setValue(instructionDraft);
    await scriptStage.click();
    await $(dialog).waitForDisplayed();
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    assert.equal(await adaptationStage.getAttribute("aria-selected"), "true");
    assert.equal(await instruction.getValue(), instructionDraft, "Cancel must preserve the generation instruction");
    await scriptStage.click();
    await $(dialog).waitForDisplayed();
    await $(dialog).$('.//button[normalize-space()="放弃草稿"]').click();
    await browser.waitUntil(async () => (await scriptStage.getAttribute("aria-selected")) === "true");

    const editor = await $(galgameEditor);
    const backgroundDetails = await editor.$("details:not([data-scene-meta-disclosure])");
    const backgroundSummary = await backgroundDetails.$("summary");
    assert((await backgroundSummary.getText()).includes("背景"));
    await backgroundSummary.click();
    const binderDetails = await backgroundDetails.$("details");
    await binderDetails.$("summary").click();
    const mediaId = await binderDetails.$('input[aria-label="素材编号"]');
    await mediaId.setValue("123456");
    const materialsStage = await $('button[id="galgame-stages-5-tab-materials"]');
    await materialsStage.click();
    await $(dialog).waitForDisplayed();
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    assert.equal(await mediaId.getValue(), "123456", "Cancel must preserve the local asset id");
    await materialsStage.click();
    await $(dialog).waitForDisplayed();
    await $(dialog).$('.//button[normalize-space()="放弃草稿"]').click();
    await browser.waitUntil(async () => (await materialsStage.getAttribute("aria-selected")) === "true");
    await scriptStage.click();
    await browser.waitUntil(async () => (await scriptStage.getAttribute("aria-selected")) === "true");
    const reopenedBackground = await $(galgameEditor).$("details:not([data-scene-meta-disclosure])");
    await reopenedBackground.$("summary").click();
    const reopenedBinder = await reopenedBackground.$("details");
    await reopenedBinder.$("summary").click();
    assert.equal(await reopenedBinder.$('input[aria-label="素材编号"]').getValue(), "");
    record.interaction = { activeStageClickPreservedCgDraft: true, instructionGuarded: true, assetIdDraftGuarded: true };
  });

  it("UX-B72 protects material-generation settings through the next-stage shortcut", async function () {
    const results = [];
    for (const width of [1440, 390]) {
      this.mediaRecord = { id: `UX-B72-${width}`, title: "素材下一阶段草稿保护", width, task: "下一阶段入口沿用统一的未保存修改确认" };
      await browser.setWindowSize(width, 1000);
      await openVideoMaterials();
      await openDetails('//details[summary//span[normalize-space()="生成设置"]]');
      const style = await $('//label[span[normalize-space()="风格模板"]]/button[@role="combobox"]');
      await selectOption(style, "自定义/后端默认");

      const nextStage = await buttonText("下一阶段");
      await nextStage.click();
      await $('[role="dialog"]').waitForDisplayed();
      assert((await $('[role="dialog"]').getText()).includes("未保存修改"));
      await $('[role="dialog"]').$('.//button[normalize-space()="取消"]').click();
      assert.equal(await style.getText(), "自定义/后端默认", "Cancel must preserve the selected material style");

      const stageControl = width < 640
        ? await $('[role="combobox"][id="video-stages-3-select"]')
        : await $('button[id="video-stages-3-tab-materials"]');
      if (width < 640) assert((await stageControl.getText()).includes("基础素材"));
      else assert.equal(await stageControl.getAttribute("aria-selected"), "true");

      await nextStage.click();
      await $('[role="dialog"]').waitForDisplayed();
      await $('[role="dialog"]').$('.//button[normalize-space()="放弃草稿"]').click();
      if (width < 640) {
        await browser.waitUntil(async () => (await stageControl.getText()).includes("导演编排"));
      } else {
        await browser.waitUntil(async () => (await $('button[id="video-stages-3-tab-director"]').getAttribute("aria-selected")) === "true");
      }
      results.push({ width, cancelPreservedStyleSelection: true, confirmOpenedDirectorStage: true });
    }
    this.mediaRecord.interaction = results;
  });

  it("UX-B73 preserves duration drafts and fractional time-block edits", async function () {
    const results = [];
    for (const width of [1440, 390]) {
      this.mediaRecord = { id: `UX-B73-${width}`, title: "视频数值字段键盘编辑", width, task: "清空并重输视频时长和带小数的分段时间" };
      await browser.setWindowSize(width, 1000);
      await openSegment(1);

      const duration = await $('//div[@role="dialog"]//label[.//span[normalize-space()="视频时长"]]//input');
      assert.equal(await duration.isExisting(), true, "The range-duration field must be available in the selected workflow");
      const originalDuration = await duration.getValue();
      await duration.click();
      await browser.keys(["CTRL", "A"]);
      await browser.keys("BACKSPACE");
      assert.equal(await duration.getValue(), "", "An empty duration must remain a draft while the user edits");
      await duration.setValue("12");
      assert.equal(await duration.getValue(), "12");
      await duration.setValue(originalDuration);

      await openVideo("片段规划");
      await buttonText("编辑规划").click();
      const segment = await $('section[aria-label="视频片段规划"] fieldset > details');
      if (!(await segment.getProperty("open"))) await segment.$("summary").click();
      const performance = await segment.$('.//details[.//summary[contains(normalize-space(.),"表演与时间设计")]]');
      if (!(await performance.getProperty("open"))) await performance.$("summary").click();
      const timeBlocks = await performance.$('.//details[.//summary[normalize-space(.)="时间段"]]');
      if (!(await timeBlocks.getProperty("open"))) await timeBlocks.$("summary").click();
      const start = await timeBlocks.$('.//label[.//span[normalize-space(.)="起始秒"]]/input');
      const end = await timeBlocks.$('.//label[.//span[normalize-space(.)="结束秒"]]/input');
      for (const [field, decimal] of [[start, "1.5"], [end, "2.5"]]) {
        const original = await field.getValue();
        await field.click();
        await browser.keys(["CTRL", "A"]);
        await browser.keys("BACKSPACE");
        assert.equal(await field.getValue(), "", "An empty time draft must remain visible during entry");
        await field.setValue(decimal);
        assert.equal(await field.getValue(), decimal, "Fractional seconds must survive controlled rerenders");
        await field.setValue(original);
      }
      await browser.saveScreenshot(`${output}/${phase}-UX-B73-${width}.png`);
      results.push({ width, emptyDraftPreserved: true, fractionalSecondsPreserved: true });
    }
    this.mediaRecord.interaction = results;
  });
});
