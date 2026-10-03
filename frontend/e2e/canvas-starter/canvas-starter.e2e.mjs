import assert from "node:assert/strict";
import { mkdirSync } from "node:fs";
import { resolve } from "node:path";

const reportDir = resolve(process.cwd(), "../target/browser-canvas-starter");

async function measureOverflow(width, height, screenshotName) {
  await browser.setWindowSize(width, height);
  await browser.pause(250);
  const measurement = await browser.execute(() => ({
    viewport: document.documentElement.clientWidth,
    scroll: document.documentElement.scrollWidth,
  }));
  assert.equal(
    measurement.scroll - measurement.viewport,
    0,
    `unexpected horizontal overflow at ${width}px: ${JSON.stringify(measurement)}`,
  );
  await browser.saveScreenshot(resolve(reportDir, screenshotName));
}

describe("independent video canvas starter", () => {
  it("creates a text-to-video graph and binds a project text asset", async () => {
    mkdirSync(reportDir, { recursive: true });
    await browser.url("/e2e/canvas-starter/");
    await browser.waitUntil(async () => (await browser.$("h1").getText()) === "独立视频工作流");

    await measureOverflow(1440, 900, "empty-1440.png");
    await measureOverflow(1024, 820, "empty-1024.png");
    await measureOverflow(390, 844, "empty-390.png");

    await browser.setWindowSize(1440, 900);
    const useTemplate = await browser.$(
      `//button[normalize-space()="使用文本到视频模板"]`,
    );
    await useTemplate.waitForClickable();
    await useTemplate.click();
    await browser.waitUntil(async () =>
      (await browser.execute(() => window.__A4N_CANVAS_STARTER_E2E__.getGraph().nodes.length)) === 4,
    );

    const graph = await browser.execute(() => window.__A4N_CANVAS_STARTER_E2E__.getGraph());
    assert.deepEqual(
      graph.nodes.map((node) => node.node_kind),
      ["TextInputNode", "LlmTextNode", "GenerateVideoNode", "VideoOutputNode"],
    );
    assert.equal(graph.edges.length, 3);
    await browser.saveScreenshot(resolve(reportDir, "text-to-video-1440.png"));

    await (await browser.$('button[aria-label="文本素材"]')).click();
    const dialog = await browser.$('div[role="dialog"]');
    await dialog.waitForDisplayed();
    await browser.waitUntil(async () => !(await dialog.$('[role="alert"]').isExisting()));
    const title = await dialog.$("input");
    await title.setValue("短片脚本");
    const textareas = await dialog.$$('textarea');
    await textareas[1].setValue("第一场：主角走进雨中的车站。\n第二场：主角发现一封旧信。");
    await (await dialog.$('//button[normalize-space()="保存"]')).click();
    await browser.waitUntil(async () =>
      (await browser.execute(() => window.__A4N_CANVAS_STARTER_E2E__.getTextAssets().length)) === 1,
    );
    await (await dialog.$('button[aria-label="关闭"]')).click();
    await dialog.waitForExist({ reverse: true });

    const sourceNode = await browser.$('.react-flow__node[data-id="1"]');
    await sourceNode.waitForClickable();
    await sourceNode.click();
    const sourceSelect = await browser.$('button[role="combobox"]');
    await sourceSelect.waitForDisplayed();
    await sourceSelect.click();
    const sourceOption = await browser.$(
      '//div[@role="option" and contains(normalize-space(), "短片脚本")]'
    );
    await sourceOption.waitForClickable();
    await sourceOption.click();
    await browser.waitUntil(async () =>
      await browser.execute(() =>
        [...document.querySelectorAll('[role="status"]')].some((item) =>
          item.textContent?.includes("短片脚本") && item.textContent.includes("第 1 版"),
        ),
      ),
    );
    await browser.waitUntil(async () =>
      (await browser.$('button[role="combobox"]').getText()).includes("短片脚本"),
    );
    const saveCanvas = await browser.$('//button[contains(normalize-space(), "保存画布")]');
    await saveCanvas.waitForClickable();
    await saveCanvas.click();
    await browser.waitUntil(async () =>
      (await browser.execute(() => window.__A4N_CANVAS_STARTER_E2E__.getGraph().nodes
        .some((node) => node.ref_type === "canvas_text_asset" && node.ref_id === 1))),
    );
    await browser.pause(4500);
    await browser.saveScreenshot(resolve(reportDir, "bound-text-asset-1440.png"));

    await measureOverflow(390, 844, "bound-text-asset-390.png");
  });
});
