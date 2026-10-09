import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";

const artifacts = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/test-center/", import.meta.url));
const settingsSeed = JSON.parse(
  await readFile(new URL("./settings-fixtures.json", import.meta.url), "utf8"),
);
const matrix = JSON.parse(
  await readFile(new URL("../../../scripts/tests/test_matrix.json", import.meta.url), "utf8"),
);
const runId = "123e4567-e89b-42d3-a456-426614174000";
const group = matrix.test_center.groups.find((item) => item.id === "all-functional");
const timeoutFor = (item) => item.suites.reduce(
  (total, suite) => total + matrix.suites[suite].timeout_seconds + 60,
  300,
) + 70 + 60;
const scenarios = matrix.test_center.groups.map((item) => ({
  id: `repository_test_${item.id}`,
  version: 1,
  category: item.category,
  title: item.title,
  description: item.description,
  effect: "test",
  target_modes: ["repository"],
  timeout_seconds: timeoutFor(item),
  parameter_schema: { type: "object", properties: {}, additionalProperties: false },
}));
const summary = {
  id: runId,
  scenario_id: `repository_test_${group.id}`,
  scenario_version: 1,
  status: "passed",
  target: { mode: "repository" },
  target_project_id: null,
  created_by: 1,
  created_at: "2026-10-09T12:00:00Z",
  started_at: "2026-10-09T12:00:00Z",
  finished_at: "2026-10-09T12:00:01Z",
  duration_ms: 1000,
  message: "测试通过",
  impact: [],
};
const testFixtures = {
  ...visualFixtures,
  ...settingsSeed.invoke,
  pageSessions: settingsSeed.rest.pageSessions,
  list_test_scenarios: scenarios,
  list_test_runs: [],
  get_test_run: {
    summary,
    parameters: {},
    steps: group.suites.map((name, index) => ({
      index,
      name,
      status: "passed",
      started_at: "2026-10-09T12:00:00Z",
      finished_at: "2026-10-09T12:00:01Z",
      message: "通过",
    })),
    logs: [],
  },
  start_test_run: summary,
};

describe("Admin test center feature flow", () => {
  before(async () => {
    await mkdir(artifacts, { recursive: true });
    await installUiFixtures(browser, testFixtures);
    await browser.url("/settings#admin");
    try {
      await $('button*=打开测试中心').waitForDisplayed();
    } catch (error) {
      const state = await browser.execute(() => ({
        url: location.href,
        text: document.body.innerText,
        errors: window.__uiAudit?.errors,
        unexpected: window.__uiAudit?.unexpected,
        commands: window.__uiAudit?.commands,
      }));
      await browser.saveScreenshot(`${artifacts}/settings-entry-failure.png`);
      console.error(JSON.stringify(state));
      throw error;
    }
  });

  it("opens grouped functional suites, starts them, and fits desktop and phone widths", async () => {
    const originalHandle = await browser.getWindowHandle();
    const handlesBeforeOpen = await browser.getWindowHandles();
    await $('button*=打开测试中心').click();
    await browser.waitUntil(async () => (await browser.getWindowHandles()).length > handlesBeforeOpen.length, {
      timeout: 5000,
      timeoutMsg: "设置页应打开独立测试中心窗口",
    });
    const handles = await browser.getWindowHandles();
    const testCenterHandle = handles.find((handle) => handle !== originalHandle);
    assert.ok(testCenterHandle);
    await browser.switchToWindow(testCenterHandle);
    await installUiFixtures(browser, testFixtures);
    await browser.refresh();
    try {
      await $("h1=功能测试中心").waitForDisplayed();
    } catch (error) {
      const state = await browser.execute(() => ({
        url: location.href,
        text: document.body.innerText,
        errors: window.__uiAudit?.errors,
        unexpected: window.__uiAudit?.unexpected,
        commands: window.__uiAudit?.commands,
      }));
      console.error(JSON.stringify({ handles, testCenterHandle, state }));
      await browser.saveScreenshot(`${artifacts}/test-center-window-failure.png`);
      throw error;
    }

    const allFunctionalButton = await $(`button*=${group.title}`);
    await allFunctionalButton.waitForDisplayed();
    await allFunctionalButton.click();
    assert.equal(await allFunctionalButton.getAttribute("aria-pressed"), "true");

    for (const width of [1440, 1024, 390]) {
      await browser.setWindowSize(width, 900);
      await browser.pause(150);
      const state = await browser.execute(() => ({
        viewport: document.documentElement.clientWidth,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      }));
      assert.equal(state.viewport, width);
      assert.equal(state.overflow, 0, `Test center must fit at ${width}px`);
      await browser.saveScreenshot(`${artifacts}/test-center-${width}.png`);
    }

    await $('button*=运行测试').click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.requests.some((request) => request.command === "start_test_run"),
    ));
    const request = await browser.execute(() =>
      window.__uiAudit.requests.find((item) => item.command === "start_test_run"),
    );
    assert.equal(request.body.input.scenario_id, `repository_test_${group.id}`);
    assert.deepEqual(request.body.input.target, { mode: "repository" });
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.requests.some((item) => item.command === "get_test_run"),
    ));
    const pageText = await browser.$("main").getText();
    assert.ok(pageText.includes("测试通过"));
    for (const suite of group.suites) assert.ok(pageText.includes(suite), `Missing suite result: ${suite}`);
    assert.deepEqual(
      (await browser.execute(() => window.__uiAudit.unexpected)),
      [],
    );

    await browser.closeWindow();
    await browser.switchToWindow(originalHandle);
  });
});
