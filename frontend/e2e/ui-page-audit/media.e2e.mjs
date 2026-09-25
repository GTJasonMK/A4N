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
  if (!(await details.getProperty("open"))) await details.$("summary").click();
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
  const chapterButtons = await $('nav[aria-label="漫画章节"]').$$("button");
  assert.equal(chapterButtons.length, 3, "The comic audit needs all three source chapters");
  await chapterButtons[chapter - 1].click();
  await $('nav[aria-label="漫画页列表"]').waitForDisplayed();
  await selectComicPage(page);
  await buttonText(view === "image" ? "成图" : "提示词与参考图").click();
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
  const target = await $(`//*[@role="tab" and normalize-space()=${JSON.stringify(tab)}]`);
  await target.waitForClickable();
  await target.click();
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
  await openVideo("素材生成");
  const target = await $('article[data-state="ready"] button[aria-label="打开素材图"]');
  await target.waitForClickable();
  await target.click();
  await $(dialog).waitForDisplayed();
  await $(`${dialog} [aria-label="图片详情栏目"]`).$(`.//button[normalize-space()=${JSON.stringify(tab)}]`).click();
  await ready();
}

async function openGalgame(view = "editor") {
  await openRoute("/galgame-project/5");
  await browser.waitUntil(async () => (await $$(galgameLines)).length === 8, {
    timeoutMsg: "The first Galgame scene must contain eight real line editors",
  });
  if (await $('[aria-label="GALGame 移动工作区"]').isDisplayed()) {
    await $('[aria-label="GALGame 移动工作区"]').$(`.//button[normalize-space()="${view === "editor" ? "编辑" : "场次"}"]`).click();
  }
  await ready();
}

async function openGalgameAssets() {
  await openGalgame("scenes");
  const entry = await $('section[aria-label="资产规划"] > button');
  await entry.waitForClickable();
  await entry.click();
  await $(dialog).waitForDisplayed();
  const plan = await $(dialog).$('.//button[normalize-space()="规划视觉资产"]');
  await plan.waitForClickable();
  await plan.click();
  await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.commands.includes("plan_galgame_assets")), {
    timeout: 2500, timeoutMsg: "The asset planning click did not issue its real fixture request",
  });
  await browser.waitUntil(async () => (await $$(`${dialog} tbody td`)).length === 12, {
    timeout: 2500, timeoutMsg: "The asset planning dialog must render all twelve portrait cells",
  });
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
    ? 'button[aria-label="同步源文本"]'
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
  await openRoute("/video-project/6/studio");
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
    const properties = await $('button[aria-label="打开属性面板"]');
    await properties.waitForClickable();
    await properties.click();
    await $(dialog).waitForDisplayed();
  }
  await ready();
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
    await $('button[aria-label="打开属性面板"]').click();
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
    return {
      reached, scrollable: Boolean(scroll), target: { x, y }, bounds,
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
  { id: "MC01", title: "漫画来源章节", task: "辨认三个来源章节及其生成状态", open: () => openRoute("/comic-project/2"), expected: ["雨夜来信", "纸上的地图", "黎明的站台"], count: ['nav[aria-label="漫画章节"] button', 3], note: "窄屏章节轨道隐藏滚动条，检查末章发现性。" },
  { id: "MC02", title: "漫画整页提示词与参考", task: "从选页进入提示词、角色参考与布局信息", open: () => openComic(), expected: ["参考图绑定", "林岚", "周衡"], count: ['nav[aria-label="漫画页列表"] button', 6], note: "检查章节状态、页状态与操作栏是否挤压正文。" },
  { id: "MC03", title: "漫画提示词编辑", task: "修改提示词并确认保存入口及取消恢复", open: async () => { await openComic(); await openMenu(comicPrompt, "编辑提示词"); await $(`${comicPrompt} textarea`).addValue("\n审查草稿"); }, expected: ["保存", "取消"], after: async () => { await $(comicPrompt).$('.//button[normalize-space()="取消"]').click(); assert.equal(await $(`${comicPrompt} textarea`).isExisting(), false); return { cancelled: true }; }, note: "保留草稿取消，不触发下游写回。" },
  { id: "MC04", title: "漫画缺失角色参考", task: "查看第4页缺失的角色绑定及候选", open: () => openComic(4), expected: ["参考图绑定", "周衡"], tail: '(//nav[@aria-label="漫画页列表"]/following-sibling::section/article//button[@role="combobox"])[last()]', note: "候选选择后的图像是否仍显示旧绑定，需要视觉复核。" },
  { id: "MC05", title: "漫画成图", task: "阅读完整漫画页并查看尺寸", open: () => openComic(1, "image"), scope: comicPage, expected: ["漫画成图"], images: 1, note: "保留图像原比例，检查媒体区域与重复状态的比例。" },
  { id: "MC06", title: "漫画灯箱", task: "完整查看纵图、放大并重置后返回页内", open: async () => { await openComic(1, "image"); await $(`${comicPage} img`).click(); await $(dialog).waitForDisplayed(); }, scope: dialog, images: 1, expected: ["100%"], after: async (record, width) => {
    const geometry = () => browser.execute(() => {
      const image = document.querySelector('.a4n-image-lightbox-image');
      const stage = image.closest('.a4n-image-lightbox-stage').getBoundingClientRect();
      const body = image.closest('.a4n-image-lightbox-body').getBoundingClientRect();
      const box = image.getBoundingClientRect();
      const inside = bounds => box.left >= bounds.left - 1 && box.right <= bounds.right + 1 && box.top >= bounds.top - 1 && box.bottom <= bounds.bottom + 1;
      return {
        width: box.width, height: box.height,
        naturalWidth: image.naturalWidth, naturalHeight: image.naturalHeight,
        scale: new DOMMatrixReadOnly(getComputedStyle(image).transform).a,
        insideStage: inside(stage), insideBody: inside(body),
      };
    });
    const assertFit = value => {
      assert(value.naturalWidth > 0 && value.naturalHeight > 0, "The lightbox image must be decoded");
      assert(Math.abs(value.scale - 1) < 0.001, "The fitted view must be at 100% scale");
      assert.equal(value.insideStage, true, "The full portrait image must fit inside the stage at 100%");
      assert.equal(value.insideBody, true, "The modal body must not clip the portrait image at 100%");
      assert(Math.abs(value.width / value.height - value.naturalWidth / value.naturalHeight) < 0.001, "The image aspect ratio must remain unchanged");
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
    assert.equal(await $('nav[aria-label="漫画页列表"] button:last-child').getAttribute("aria-current"), "true");
    assert.equal(await $(`${comicPrompt} pre`).getText(), comicFixture(6).image_prompt);
    return { selectedPage: 6, promptId: 21006, currentTextMatches: true };
  }, note: "末页通过真实 wheel 后点击，核对当前页完整文字与过期状态；不依赖已删除的重复区块标题。" },
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

  { id: "MV01", title: "视频素材", task: "核对场景和物体的正常、缺失和过期素材", open: () => openVideo("素材生成"), count: ["article[data-state]", 6], expected: ["旧书店", "旧怀表"], note: "素材主操作是否藏在默认折叠的生成设置内。" },
  { id: "MV02", title: "素材生成设置", task: "查看尺寸、风格及批量补全入口", open: async () => { await openVideo("素材生成"); await $('//summary[.//span[normalize-space()="生成设置"]]').click(); }, expected: ["补齐", "缺失"], note: "尺寸与风格可折叠，主要补全任务应易发现。" },
  { id: "MV03", title: "素材缺失筛选", task: "用已有筛选仅查看缺失素材", open: async () => { await openVideo("素材生成"); const trigger = await $('//label[.//span[normalize-space()="状态"]]//button[@role="combobox"]'); await selectOption(trigger, "缺失"); }, count: ["article[data-state]", 2], note: "真实筛选结果应改变，不用相同数组冒充筛选成功。" },
  { id: "MV04", title: "素材上传对话框", task: "查看上传前的角色/场景归属与主图开关", open: async () => { await openVideo("素材生成"); await openMenu('article[data-state="ready"]', "上传素材图"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["上传", "主"], note: "保留文件选择和上传后设为主图；本例不上传文件。" },
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
  ].map(([id, number, mode, task]) => ({ id, title: `片段${number}·${mode}`, task, open: () => openSegment(number), scope: dialog, expected: [mode], after: ["MV12", "MV13", "MV14", "MV15", "MV16"].includes(id) ? inspectVideoEditorEnd : undefined, note: "五种已就绪制作模式均真实滚到末尾文字字段并记录interaction截图，不能以首屏模式按钮替代编辑可达性。" })),
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
  { id: "MV30", title: "成片合成", task: "审查四个可用片段的顺序与已推送前缀", open: async () => { await openVideo("成片合成"); await waitForPlayable(".a4n-page-container video"); }, expected: ["成片预览", "片段顺序"], after: () => provePlayback(".a4n-page-container video"), note: "检查确认操作与顺序列表的距离，预览需包含真实媒体。" },
  { id: "MV31", title: "后处理批次", task: "从两批成片进入待确认的编辑版本", open: () => openPostprocess(0), expected: ["推送批次", "后处理预览", "剪辑条"], count: ["[data-video-push-id]", 2], note: "播放器不应把实际编辑时间条压到不可见处。" },
  { id: "MV32", title: "后处理插入点", task: "点击时间条设置一个插入点", open: () => openPostprocess(1), expected: ["上传并插入"], count: [`${timeline} [role="slider"]`, 1], note: "命中区、时间读数和撤销选择入口需明确。" },
  { id: "MV33", title: "后处理选区", task: "用两个标记定义可替换的片段", open: () => openPostprocess(2), expected: ["上传并替换"], count: [`${timeline} [role="slider"]`, 2], note: "检查双标记间距、选区高亮和长时间读数的碰撞。" },
  { id: "MV34", title: "后处理键盘微调", task: "通过方向键精确移动时间标记", open: () => openPostprocess(1), expected: ["剪辑条"], after: async () => { const marker = await $(`${timeline} [role="slider"]`); const before = Number(await marker.getAttribute("aria-valuenow")); await marker.execute(element => element.focus()); await browser.keys("ArrowRight"); const after = Number(await marker.getAttribute("aria-valuenow")); assert(after > before, "ArrowRight must move the selected marker"); return { before, after }; }, note: "保留0.05秒方向键微调与大步移动能力。" },
  { id: "MV35", title: "帧图点击配置", task: "点击首帧图片中央进入配置", open: async () => { await openSegment(1); await $(`${dialog} button[aria-label="配置首帧"] img`).click(); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "首帧配置", { timeout: 2500, timeoutMsg: "The frame image must activate configuration" }); }, scope: dialog, expected: ["首帧配置"], images: 1, note: "图片主体属于配置入口，不能被预览组件截断。" },
  { id: "MV36", title: "帧图独立预览", task: "用独立预览按钮放大首帧并返回配置页", open: async () => { await openSegment(1); await $(`${dialog} button[aria-label="图片预览"]`).click(); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["100%"], images: 1, after: async () => { await browser.keys("Escape"); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "片段 1 · 配置"); return { returnedToSegmentConfig: true }; }, note: "独立放大保留，关闭只退出顶部灯箱。" },
  { id: "MV37", title: "插入参考图独立预览", task: "放大参考图时保持原选择状态", open: openInserted, scope: dialog, expected: ["生成插入片段"], after: async (record, width) => { const option = await $(`${dialog} button:has(img)`); const selected = (await option.getAttribute("class")).includes("ring-1"); await $(`${dialog} button[aria-label="图片预览"]`).click(); await browser.waitUntil(async () => (await $(dialog).getText()).includes("100%")); record.lightbox = await capture({ id: "MV37", scope: dialog }, width, "-lightbox"); await browser.keys("Escape"); await browser.waitUntil(async () => (await $(`${dialog} h3`).getText()) === "生成插入片段"); assert.equal((await option.getAttribute("class")).includes("ring-1"), selected); return { selectionUnchanged: true }; }, note: "放大与选择分别验证，不能以灯箱打开替代选择成功。" },

  { id: "MG01", title: "Galgame场次与恢复", task: "查看三个场次、来源与失败恢复", open: () => openGalgame("scenes"), expected: ["铜铃之后的来信", "纸背上的另一条路", "第三站台的约定"], note: "场次长列表和状态面板不能淹没素材制作入口。" },
  { id: "MG02", title: "Galgame混合台词", task: "阅读八行不同类型的剧本", open: () => openGalgame(), count: [galgameLines, 8], expected: ["林岚", "周衡"], note: "重复的姿态、表情、CG和转场控件是否打断台词阅读。" },
  { id: "MG03", title: "Galgame背景绑定", task: "核对场景背景与既有素材绑定入口", open: async () => { await openGalgame(); await openDetails(`${galgameEditor} > div:nth-child(2) details`); }, scope: galgameEditor, images: 1, expected: ["背景"], note: "背景、当前素材与编号输入应有清楚关联。" },
  { id: "MG04", title: "Galgame对白", task: "查看说话角色、台词与姿态表情", open: () => openGalgamePerformance(2), tail: `${galgameLines}:nth-of-type(2) button[aria-label="表情"]`, expected: ["林岚", "表情"], note: "演出默认收起；实际展开后验证持久字段名与当前值。" },
  { id: "MG05", title: "Galgame独白与演出", task: "对照独白和舞台指示的编辑差异", open: () => openGalgame(), tail: `${galgameLines}:nth-of-type(5) textarea`, expected: ["独白", "演出"], note: "保留五种行类型各自字段，不以同一个空行代表。" },
  { id: "MG06", title: "Galgame末行选择支", task: "滚到第8行确认分支目标与合流", open: () => openGalgame(), tail: '//section[@aria-label="编辑"]//article[8]//button[normalize-space()="保存选择支"]', expected: ["保存选择支"], note: "900px是外层与内层滚动接管的重点断点。" },
  { id: "MG07", title: "GalgameCG与转场", task: "查看启用CG的台词及演出说明", open: () => openGalgamePerformance(6), tail: '//section[@aria-label="编辑"]//article[6]//textarea[@placeholder="事件 CG 画面描述"]', expected: ["CG"], note: "实际展开演出；已启用CG保留默认展开，正文仍是视觉主体。" },
  { id: "MG08", title: "Galgame音频媒体", task: "展开台词媒体并播放已有BGM", open: async () => { await openGalgame(); await openDetails(`${galgameLines}:first-of-type > details:not([class*="group/performance"])`); }, scope: galgameEditor, expected: ["BGM", "SFX"], after: async () => {
    const audio = `${galgameLines}:first-of-type audio`;
    assert.equal((await reachByWheel(audio)).reached, true, "The native BGM player must be visibly reachable before playback");
    return provePlayback(audio);
  }, note: "先真实滚到BGM控制区再验证播放时间推进，保留可见音频控件的interaction截图。" },
  { id: "MG09", title: "Galgame视觉资产计划", task: "查看6背景、12立绘单元和2CG", open: openGalgameAssets, scope: dialog, expected: ["角色差分", "CG", "站台·镜中出口"], count: [`${dialog} tbody td`, 12], note: "资产已集中到独立弹窗；检查全部背景、矩阵和固定底栏，而不是旧侧栏布局。" },
  { id: "MG10", title: "Galgame矩阵末项", task: "找到矩阵最右列并读取生成状态", open: async () => { await openGalgameAssets(); await reachByWheel(`${dialog} tbody tr:last-child td:last-child button`, "y"); }, scope: dialog, tail: `${dialog} tbody tr:last-child td:last-child button`, axis: "x", expected: ["视觉资产", "生成素材"], note: "单格动作准确标为生成素材；窄屏保留真实横向滚动与大图预览。" },
  { id: "MG11", title: "Galgame生成选项", task: "查看可续跑阶段与生成起点", open: async () => { await openGalgame("scenes"); await openDetails('details[aria-label="剧本生成"]'); await openDetails('details[aria-label="剧本生成"] details'); }, expected: ["生成", "从头开始"], note: "可恢复失败状态与生成参数必须同时可辨。" },
  { id: "MG12", title: "Galgame请求预览", task: "从生成选项查看完整请求", open: async () => { await openGalgame("scenes"); await openDetails('details[aria-label="剧本生成"]'); await openMenu('details[aria-label="剧本生成"]', "查看模型请求"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["请求"], note: "保留阶段、来源、提示词和退出路径。" },
  { id: "MG13", title: "Galgame重置确认", task: "审查重置影响范围后安全取消", frequency: "低", open: async () => { await openGalgame(); await openMenu(".a4n-page-header", "重置生成数据"); await $(dialog).waitForDisplayed(); }, scope: dialog, expected: ["不可恢复", "重置"], after: async () => { assert.equal(await $(`${dialog} button[aria-label="关闭"]`).isDisplayed(), true); await browser.keys("Escape"); await $(dialog).waitForExist({ reverse: true }); assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("reset_project_generated_data")), false); return { cancelledBeforeMutation: true }; }, note: "使用既有关闭/Escape安全取消，不要求新增底栏取消按钮。" },

  { id: "MX01", title: "画布保存视角", task: "按已保存视角阅读节点与主要数据流", open: () => openCanvas(), widths: canvasWidths, count: [".react-flow__node", 17], expected: ["节点"], note: "图中17节点均已加载；初始呈现遵守保存视角，不声称所有节点同时在屏内。" },
  ...[
    ["MX02", 6001, "文本输入", "文本"], ["MX03", 6013, "LLM节点", "指令"],
    ["MX04", 6003, "图片生成节点", "图片配置"], ["MX05", 6005, "视频生成节点", "输入模式"],
    ["MX06", 6017, "缺失首尾帧节点", "首尾帧"], ["MX07", 6008, "组装顺序", "成片轨道"],
  ].map(([id, node, title, text]) => ({ id, title, task: `选择${title}并修改前核对属性`, open: () => openCanvas(node), widths: canvasWidths, expected: [text], note: "1024px属性必须有入口；参数、输入与输出分组和字号待复核。" })),
  { id: "MX08", title: "画布工作流", task: "读取已完成节点、失败原因及恢复入口", open: () => openCanvas(6008), widths: canvasWidths, expected: ["结尾镜头缺少首帧与尾帧", "运行状态"], note: "运行状态位于属性尾部，长参数下的可见性需检查。" },
  ...[
    ["MX09", 6014, "文本核验", "text"], ["MX10", 6016, "尾帧核验", "image"],
    ["MX11", 6009, "成片输出", "video"], ["MX12", 6011, "雨声监听", "audio"],
  ].map(([id, node, title, kind]) => ({ id, title: `画布${kind}内容`, task: `打开${title}的已有内容`, open: () => openCanvasContent(node, title), widths: canvasWidths, scope: dialog, expected: [title], images: kind === "image" ? 1 : 0, after: kind === "video" || kind === "audio" ? () => provePlayback(`${dialog} ${kind}`) : undefined, note: "使用真实内容与媒体解码，不能由节点占位缩略图替代。" })),
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
    const fields = await $(await canvasInspector()).$$('input[type="number"]');
    assert.equal(fields.length, 2);
    assert.equal(Number(await fields[0].getValue()), 0);
    assert.equal(Number(await fields[1].getValue()), 6);
    assert.equal((await reachByWheel(fields[1])).reached, true);
    await fields[1].setValue("5");
    assert.equal(Number(await fields[1].getValue()), 5);
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("save_canvas_graph")), false);
    return { nodeId: 6007, trimStart: 0, originalEnd: 6, draftEnd: 5, saved: false };
  }, note: "裁剪属性独立验收，末项可达且真实接受本地输入；不执行裁剪或保存。" },
  { id: "MX17", title: "画布指定时间抽帧", task: "从尾帧切换为指定时间并输入时间点", open: async () => {
    await openCanvas(6015);
    const scope = await canvasInspector();
    const kind = await $(scope).$('button[role="combobox"]');
    assert.equal(await kind.getText(), "尾帧");
    await selectOption(kind, "指定时间");
    const seconds = await $(scope).$('input[type="number"]');
    assert.equal((await reachByWheel(seconds)).reached, true);
    await seconds.setValue("2");
  }, widths: canvasWidths, expected: ["指定时间", "时间秒"], after: async () => {
    assert.equal(Number(await $(await canvasInspector()).$('input[type="number"]').getValue()), 2);
    assert.equal(await browser.execute(() => window.__uiAudit.commands.some(command => ["save_canvas_graph", "run_canvas_node"].includes(command))), false);
    return { nodeId: 6015, specifiedFrameSeconds: 2, saved: false };
  }, note: "验证尾帧到指定时间的实际切换，字段可达；不运行抽帧或保存。" },
  { id: "MX18", title: "画布已有视频引用", task: "展开当前视频输入并核对资源和缩略图", open: async () => { await openCanvas(6006); await openDetails(`${await canvasInspector()} details`); }, widths: canvasWidths, expected: ["视频资源编号"], after: async () => {
    const scope = await canvasInspector();
    const field = await $(scope).$('input[type="number"]');
    const source = fixtures.byProject[6].get_canvas_graph.nodes.find(node => node.id === 6006);
    assert.equal(Number(await field.getValue()), source.ref_id);
    assert.equal((await reachByWheel(field)).reached, true);
    await field.click();
    const image = await $(scope).$('img');
    assert.equal(await image.execute(element => element.currentSrc), source.config_json.thumbnail_path);
    assert.equal(await image.execute(element => element.complete && element.naturalWidth > 0), true);
    return { nodeId: 6006, videoId: source.ref_id, thumbnailMatches: true };
  }, note: "当前编号 34001 与当前真实缩略图必须同时匹配，不以参数标题代替内容身份。" },
  { id: "MX19", title: "画布连线属性", task: "点击真实连线并核对两端端口", open: openCanvasEdge, widths: canvasWidths, expected: ["来源端口", "目标端口", "连接类型"], after: async () => {
    const details = await $(await canvasInspector()).$('details');
    const values = await details.$('div').$$(':scope > span');
    const source = fixtures.byProject[6].get_canvas_graph.edges.find(edge => edge.id === 6801);
    assert.equal(values.length, 8);
    assert.deepEqual(await Promise.all([values[1].getText(), values[3].getText(), values[5].getText()]), [source.role, source.source_port, source.target_port]);
    assert.equal((await reachByWheel(values.at(-1))).reached, true);
    return { edgeId: source.id, sourcePort: source.source_port, targetPort: source.target_port, role: source.role };
  }, note: "SVG 实际点击后逐值核对 context/text/input，并检查末项可达。" },
  { id: "MX20", title: "画布单节点运行确认", task: "核对当前运行目标后取消", open: async () => { await openCanvas(6005); if ((await browser.execute(() => innerWidth)) <= 1100) await browser.keys("Escape"); await openMenu(".a4n-page-header", "只运行所选节点"); await $(dialog).waitForDisplayed(); }, widths: canvasWidths, scope: dialog, expected: ["只运行此节点", "连续镜头", "取消"], after: async () => {
    await $(dialog).$('.//button[normalize-space()="取消"]').click();
    await $(dialog).waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("run_canvas_node")), false);
    assert((await $('.react-flow__node[data-id="6005"]').getAttribute("class")).includes("selected"));
    return { selectedNodeId: 6005, cancelledBeforeCanvasRun: true };
  }, note: "确认内容必须指向连续镜头；取消后保留节点选择，不启动执行。" },
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
    return { nodeId: 6006, imageMatches: true, selectionUnchanged: true, focusRestored: true };
  }, note: "独立记录灯箱图；Escape 后精确核对原预览按钮获得焦点，不仅检查弹窗消失。" },
  { id: "MX23", title: "画布缩放与交互锁", task: "放大、缩小、适配画布并锁定后恢复编辑", open: () => openCanvas(), widths: canvasWidths, expected: ["影像画布"], after: async (record, width) => {
    const controls = '.a4n-flow-controls';
    assert.equal((await $$(`${controls} button`)).length, 4);
    assert.equal((await $$(`${controls} button > svg.lucide`)).length, 4, "All canvas controls must use Lucide icons");
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
  ...[["MS01", 2, "漫画"], ["MS02", 3, "视频"], ["MS03", 5, "Galgame"]].map(([id, project, label]) => ({ id, title: `${label}来源同步冲突`, task: "比较三条来源变化与当前改编，关闭前不应用决定", frequency: "低", open: () => openSourceSync(project), scope: dialog, count: [`${dialog} article`, 3], expected: ["冲突清单", "源文本变化", "当前项目", "未寄出的信"], after: inspectSourceSyncEnd, note: "typed dry_run预览；真实滚到末条冲突并补充conflicts-end截图，不为应用/解绑提供假成功mutation。" })),
];

describe("Populated media pages and task dialogs", () => {
  before(async () => {
    await mkdir(output, { recursive: true });
    fixtures = await buildPageFixtures(browser.options.baseUrl);
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
        assert.deepEqual(record.errors, [], "Browser errors must remain visible");
        const readableContent = [record.text, ...record.controls.map(control => control.value ?? "")].join("\n");
        for (const expected of entry.expected ?? []) assert(readableContent.includes(expected), `${entry.id}: missing populated content ${expected}`);
        if (entry.count) assert.equal((await $$(entry.count[0])).length, entry.count[1], `${entry.id}: populated record count`);
        if (entry.images) assert(record.images.filter(image => image.loaded).length >= entry.images, `${entry.id}: no decoded visible image`);
        if (entry.tail) assert.equal(record.reachability.reached, true, `${entry.id}: last task control cannot be reached with real scrolling`);
        if (entry.after) {
          record.interaction = await entry.after(record, width);
          record.afterInteraction = await capture({ id: entry.id }, width, "-interaction");
          assert.deepEqual(record.afterInteraction.unexpected, []);
          assert.deepEqual(record.afterInteraction.errors, []);
        }
      } catch (error) {
        record.failure = error instanceof Error ? error.message : String(error);
        throw error;
      }
    });
  }
});
