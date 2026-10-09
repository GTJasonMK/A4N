import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { buildPageFixtures, seedLocalModels } from "./fixtures.mjs";

const output = fileURLToPath(new URL("../../../.dev-runtime/ui-design-20261010/media/", import.meta.url));
const widths = [1440, 1024, 390];
const modes = ["light", "dark"];
const stageIds = {
  comic: "comic-stages-2",
  video: "video-stages-3",
  galgame: "galgame-stages-5",
};
const cases = [
  { id: "comic", route: "/comic-project/2", stage: "基础素材", ready: ".a4n-media-stage-header" },
  { id: "video", route: "/video-project/3", stage: "片段生成", ready: ".a4n-media-stage-header" },
  { id: "galgame", route: "/galgame-project/5", stage: "资产规划", ready: ".a4n-media-stage-header" },
  { id: "canvas", route: "/canvas-project/6", ready: ".react-flow__node" },
];
const report = [];

async function chooseStage(page, label) {
  const select = await $(`[role="combobox"][id="${stageIds[page]}-select"]`);
  if (await select.isDisplayed()) {
    await select.click();
    const option = await $(`//*[@role="option" and contains(normalize-space(.), ${JSON.stringify(label)})]`);
    await option.waitForClickable();
    await option.click();
    return;
  }
  const tab = await $(`//*[@role="tab" and contains(normalize-space(.), ${JSON.stringify(label)})]`);
  if (await tab.isExisting() && await tab.getAttribute("aria-selected") !== "true") await tab.click();
}

async function renderCase(entry, width, mode) {
  await browser.setWindowSize(width, 1000);
  await browser.url(entry.route);
  await $(".a4n-page-header h1").waitForDisplayed({ timeout: 15_000 });
  if (entry.stage) await chooseStage(entry.id, entry.stage);
  try {
    await $(entry.ready).waitForDisplayed({ timeout: 15_000 });
  } catch (error) {
    const state = await browser.execute(() => ({
      route: location.pathname,
      text: document.body.innerText,
      unexpected: window.__uiAudit.unexpected,
      errors: window.__uiAudit.errors,
    }));
    await browser.saveScreenshot(`${output}/${entry.id}-${width}-${mode}-failure.png`);
    throw new Error(`${entry.id} visual fixture did not reach its content surface: ${JSON.stringify(state)}`, { cause: error });
  }
  if (entry.id === "canvas") {
    await browser.waitUntil(async () => (await $$(".react-flow__node")).length === 17, {
      timeout: 15_000,
      timeoutMsg: "Canvas visual review requires the populated graph fixture",
    });
  }
  await browser.execute(async (appearance) => {
    const { setThemeAppearance } = await import("/src/theme/activeTheme.ts");
    setThemeAppearance(appearance);
    await document.fonts.ready;
    await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
  }, mode);
  await browser.pause(220);
  const state = await browser.execute(() => {
    const visible = (element) => {
      const style = getComputedStyle(element);
      const box = element.getBoundingClientRect();
      return style.display !== "none" && style.visibility !== "hidden" && box.width > 1 && box.height > 1;
    };
    const controls = [...document.querySelectorAll("button,input,textarea,[role=combobox]")].filter(visible);
    const scrollableTables = [...document.querySelectorAll('[role="region"][tabindex="0"]')]
      .filter((region) => visible(region) && region.querySelector("table"))
      .map((region) => {
        const box = region.getBoundingClientRect();
        return {
          clientWidth: region.clientWidth,
          scrollWidth: region.scrollWidth,
          keyboardFocusable: region.tabIndex >= 0,
          scrollbarWidth: getComputedStyle(region).scrollbarWidth,
          withinViewport: box.left >= -1 && box.right <= innerWidth + 1,
        };
      });
    const canvasControls = document.querySelector(".a4n-studio-flow .a4n-flow-controls");
    const canvasControlsBox = canvasControls?.getBoundingClientRect();
    const nodesUnderCanvasControls = canvasControlsBox
      ? [...document.querySelectorAll(".react-flow__node")]
        .filter((node) => {
          const box = node.getBoundingClientRect();
          return box.left < canvasControlsBox.right && box.right > canvasControlsBox.left &&
            box.top < canvasControlsBox.bottom && box.bottom > canvasControlsBox.top;
        })
        .map((node) => node.getAttribute("data-id"))
      : [];
    return {
      width: innerWidth,
      height: innerHeight,
      mode: document.documentElement.dataset.themeMode,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      outside: controls.filter((element) => {
        if (element.closest(".react-flow__viewport,.a4n-mobile-scroll-rail,.a4n-filter-chips")) return false;
        const box = element.getBoundingClientRect();
        const scrollRegion = element.closest('[role="region"][tabindex="0"]');
        if (
          scrollRegion &&
          scrollRegion.scrollWidth > scrollRegion.clientWidth &&
          scrollRegion.getBoundingClientRect().left >= -1 &&
          scrollRegion.getBoundingClientRect().right <= innerWidth + 1
        ) return false;
        return box.left < -1 || box.right > innerWidth + 1;
      }).map((element) => element.getAttribute("aria-label") || element.textContent?.trim().slice(0, 80)),
      nodeCount: document.querySelectorAll(".react-flow__node").length,
      scrollableTables,
      flowControls: canvasControls ? {
        width: canvasControlsBox.width,
        height: canvasControlsBox.height,
        buttonHeight: canvasControls.querySelector("button")?.getBoundingClientRect().height ?? 0,
        flexDirection: getComputedStyle(canvasControls).flexDirection,
        withinViewport: canvasControlsBox.left >= -1 && canvasControlsBox.right <= innerWidth + 1,
      } : null,
      nodesUnderCanvasControls,
      bodyText: document.body.innerText,
      unexpected: window.__uiAudit.unexpected,
      errors: window.__uiAudit.errors,
    };
  });
  const file = `${entry.id}-${width}-${mode}.png`;
  await browser.saveScreenshot(`${output}/${file}`);
  assert.equal(state.width, width);
  assert.equal(state.mode, mode);
  assert.equal(state.overflow, 0, `${file}: horizontal overflow`);
  assert.deepEqual(state.outside, [], `${file}: controls outside the viewport`);
  assert.deepEqual(state.unexpected, [], `${file}: missing fixture`);
  assert.deepEqual(state.errors, [], `${file}: browser error`);
  assert.doesNotMatch(state.bodyText, /\p{Extended_Pictographic}/u, `${file}: UI emoji`);
  if (entry.id === "galgame" && width === 390) {
    assert.equal(state.scrollableTables.length, 1, `${file}: expression table scroll region`);
    assert.ok(state.scrollableTables[0].scrollWidth > state.scrollableTables[0].clientWidth, `${file}: expression table can scroll horizontally`);
    assert.equal(state.scrollableTables[0].keyboardFocusable, true, `${file}: expression table remains keyboard reachable`);
    assert.notEqual(state.scrollableTables[0].scrollbarWidth, "none", `${file}: horizontal scrolling remains visible`);
    assert.equal(state.scrollableTables[0].withinViewport, true, `${file}: expression table scroller stays within the viewport`);
  }
  if (entry.id === "canvas") {
    assert.equal(state.nodeCount, 17, `${file}: populated canvas nodes`);
    assert.ok(state.flowControls, `${file}: canvas controls are visible`);
    assert.equal(state.flowControls.flexDirection, "row", `${file}: canvas controls stay in one row`);
    assert.ok(state.flowControls.height <= state.flowControls.buttonHeight + 2, `${file}: canvas control panel stays compact`);
    assert.equal(state.flowControls.withinViewport, true, `${file}: canvas controls stay within the viewport`);
    assert.deepEqual(state.nodesUnderCanvasControls, [], `${file}: canvas controls do not cover nodes`);
  }
  report.push({ page: entry.id, route: entry.route, file, ...state });
}

describe("Populated media and canvas visual matrix", () => {
  before(async () => {
    await mkdir(output, { recursive: true });
    const fixtures = await buildPageFixtures(browser.options.baseUrl);
    const canvasProject = fixtures.projectRecords.find((project) => project.id === 6);
    assert.ok(canvasProject, "Canvas visual fixture requires project 6");
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
    await installUiFixtures(browser, fixtures);
    await seedLocalModels(browser);
  });

  after(async () => {
    await writeFile(`${output}/report.json`, JSON.stringify(report, null, 2));
  });

  for (const entry of cases) {
    it(`renders ${entry.id} with populated content at all target sizes`, async () => {
      for (const width of widths) {
        for (const mode of modes) await renderCase(entry, width, mode);
      }
    });
  }
});
