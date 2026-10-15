import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";

const output = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/", import.meta.url));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";

describe("Individual page workflows", () => {
  before(async () => {
    await mkdir(output, { recursive: true });
    const fixtures = structuredClone(visualFixtures);
    const canvasProject = fixtures.projectRecords.find((project) => project.id === 6);
    assert.ok(canvasProject, "画布项目 fixture is required");
    canvasProject.project_kind = "canvas";
    canvasProject.project_type = "canvas";
    fixtures.get_canvas_graph.graph.graph_kind = "video_studio";
    fixtures.list_canvas_text_assets = [];
    await installUiFixtures(browser, fixtures);
  });

  afterEach(async function () {
    await browser.saveScreenshot(`${output}/${phase}-${this.currentTest.title.split(" ")[0]}.png`);
    const issues = await browser.execute(() => ({
      unexpected: window.__uiAudit.unexpected,
      errors: window.__uiAudit.errors,
    }));
    assert.deepEqual(issues, { unexpected: [], errors: [] });
  });

  it("X01-desktop-node-library lets keyboard users add a node", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/canvas-project/6");
    await $(".react-flow__node").waitForDisplayed();
    const count = (await $$(".react-flow__node")).length;
    const template = await $('button[draggable="true"]');
    await template.execute((element) => element.focus());
    await browser.keys("Enter");
    await browser.waitUntil(async () => (await $$(".react-flow__node")).length === count + 1, {
      timeout: 2500, timeoutMsg: "Keyboard activation must add the same node as dragging it",
    });
  });

  it("X02-responsive-properties keeps selected node properties reachable on tablet and phone", async () => {
    for (const width of [1024, 390]) {
      await browser.setWindowSize(width, 1000);
      await browser.url("/canvas-project/6");
      const node = await $('.react-flow__node[data-id="1"]');
      await node.waitForDisplayed();
      await node.click();
      const trigger = await $('.a4n-video-canvas-header button[aria-label="更多操作"]');
      assert.equal(await trigger.isDisplayed(), true, `${width}px: the hidden sidebar needs a visible properties entry`);
      await trigger.click();
      const properties = await $('//*[@role="menuitem" and normalize-space(.)="打开属性面板"]');
      await properties.waitForClickable();
      await properties.click();
      await $('[role="dialog"]').waitForDisplayed();
      assert.equal(await $('[role="dialog"] textarea').isDisplayed(), true);
      await browser.keys("Escape");
      await $('[role="dialog"]').waitForExist({ reverse: true });
      assert.equal(await trigger.isFocused(), true);
    }
  });
});
