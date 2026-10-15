import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";
import { buildPageFixtures } from "./fixtures.mjs";

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
    const sharedPageFixtures = await buildPageFixtures(browser.options.baseUrl);
    Object.assign(testFixtures, {
      byProject: sharedPageFixtures.byProject,
      byChapter: sharedPageFixtures.byChapter,
    });
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

    await browser.setWindowSize(1440, 900);
    await browser.saveScreenshot(`${artifacts}/default-1440.png`);
    await browser.setWindowSize(390, 900);
    await browser.saveScreenshot(`${artifacts}/default-390.png`);
    await browser.setWindowSize(1440, 900);
    const allFunctionalButton = await $(`button*=${group.title}`);
    await allFunctionalButton.waitForDisplayed();
    await allFunctionalButton.click();
    assert.equal(await allFunctionalButton.getAttribute("aria-pressed"), "true");

    for (const width of [1440, 1280, 1279, 1024, 390]) {
      await browser.setWindowSize(width, 900);
      await browser.pause(150);
      const state = await browser.execute(() => ({
        viewport: document.documentElement.clientWidth,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      }));
      assert.equal(state.viewport, width);
      assert.equal(state.overflow, 0, `Test center must fit at ${width}px`);
      assert.equal(
        await $('select[aria-label="功能场景"]').isDisplayed(),
        width < 1280,
        `Scenario picker must match the ${width}px layout`,
      );
      if (width === 390) {
        const picker = await $('select[aria-label="功能场景"]');
        const selectedScenarioId = `repository_test_${group.id}`;
        const selectedScenario = scenarios.find((scenario) => scenario.id === selectedScenarioId);
        const alternateScenario = scenarios.find((scenario) => scenario.id !== selectedScenarioId);
        assert.ok(alternateScenario);
        const categoryPicker = await $('select[aria-label="分类"]');
        await browser.execute(() => document.querySelector('select[aria-label="分类"]')?.focus());
        await browser.keys("Tab");
        const scenarioFocus = await browser.execute(() => ({
          label: document.activeElement?.getAttribute("aria-label"),
          visible: document.activeElement?.matches(":focus-visible"),
        }));
        assert.deepEqual(scenarioFocus, { label: "功能场景", visible: true });
        await browser.keys("Tab");
        const runFocus = await browser.execute(() => ({
          text: document.activeElement?.textContent?.trim(),
          visible: document.activeElement?.matches(":focus-visible"),
        }));
        assert.deepEqual(runFocus, { text: "运行测试", visible: true });
        await picker.selectByAttribute("value", alternateScenario.id);
        assert.equal(await picker.getValue(), alternateScenario.id);
        await picker.selectByAttribute("value", selectedScenarioId);
        assert.equal(await picker.getValue(), selectedScenarioId);
        const filteredCategory = scenarios.find((scenario) => scenario.category !== selectedScenario?.category)?.category;
        if (filteredCategory) {
          await categoryPicker.selectByAttribute("value", filteredCategory);
          assert.equal(await picker.getValue(), selectedScenarioId, "Category filtering must retain the selected scenario");
          await categoryPicker.selectByAttribute("value", "all");
        }
      }
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
    const resultLayout = await browser.execute(() => ({
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      viewport: document.documentElement.clientWidth,
      visibleSteps: [...document.querySelectorAll("ol li")].filter((item) => {
        const rect = item.getBoundingClientRect();
        return rect.top >= 0 && rect.bottom <= window.innerHeight;
      }).length,
    }));
    assert.equal(resultLayout.viewport, 390);
    assert.equal(resultLayout.overflow, 0, "The test result must fit the phone viewport");
    assert.ok(resultLayout.visibleSteps > 0, "Suite details must be visible after a phone run");
    await browser.saveScreenshot(`${artifacts}/test-center-result-390.png`);
    assert.deepEqual(
      (await browser.execute(() => window.__uiAudit.unexpected)),
      [],
    );

    await browser.closeWindow();
    await browser.switchToWindow(originalHandle);
  });

  it("keeps filters, run details, and created-project cleanup aligned", async () => {
    const runId = "123e4567-e89b-42d3-a456-426614174001";
    const scenarios = [
      {
        id: "admin_overview", version: 1, category: "system", title: "管理员运行概览",
        description: "验证管理员概览读取。", effect: "read",
        target_modes: ["test_project", "current_project"], timeout_seconds: 30,
        parameter_schema: { type: "object", properties: {}, additionalProperties: false },
      },
      {
        id: "project_duplicate", version: 1, category: "projects", title: "项目复制",
        description: "复制当前项目或指定源项目。", effect: "write",
        target_modes: ["test_project", "current_project"], timeout_seconds: 30,
        parameter_schema: { type: "object", properties: {}, additionalProperties: false },
      },
    ];
    const sourceProject = testFixtures.projectRecords.find((project) => project.project_kind === "video");
    assert.ok(sourceProject, "video project record fixture is required");
    const sourceSummary = {
      ...settingsSeed.invoke.list_all_projects[0],
      id: sourceProject.id,
      title: sourceProject.title,
      project_kind: "video",
      project_type: sourceProject.project_type,
    };
    const copySummary = { ...sourceSummary, id: 99, title: "复制后的先导片" };
    const copyProject = {
      ...sourceProject,
      id: 99,
      title: copySummary.title,
      project_kind: "video",
      project_type: "video",
      source_mode: "derived",
    };
    const run = {
      ...summary,
      id: runId,
      scenario_id: "project_duplicate",
      status: "passed",
      target: { mode: "current_project", project_id: sourceSummary.id },
      target_project_id: copySummary.id,
      impact: [{ action: "create", resource: "project", project_id: copySummary.id, detail: "复制项目" }],
    };
    const queuedRun = { ...run, status: "queued", message: null };
    const runningDetail = {
      summary: { ...run, status: "running", message: null },
      parameters: {},
      steps: [{ index: 0, name: "读取复制项目", status: "running", started_at: run.created_at, finished_at: null, message: null }],
      logs: ["开始：读取复制项目"],
    };
    const passedDetail = {
      ...runningDetail,
      summary: { ...run, status: "passed" },
      steps: [{ ...runningDetail.steps[0], status: "passed", finished_at: run.finished_at, message: "通过" }],
    };
    const fixtures = {
      ...testFixtures,
      list_test_scenarios: scenarios,
      list_test_runs: [runningDetail.summary],
      get_test_run: runningDetail,
      list_all_projects: [sourceSummary],
      byProject: {
        ...testFixtures.byProject,
        [copyProject.id]: structuredClone(testFixtures.byProject[sourceProject.id]),
      },
      projectRecords: [...testFixtures.projectRecords.filter((project) => project.id !== copyProject.id), copyProject],
      deferred: ["get_test_run"],
      delete_test_run: null,
    };
    await installUiFixtures(browser, fixtures);
    await browser.setWindowSize(1440, 900);
    await browser.url(`/admin/test-center?runId=${runId}`);
    await $("h1=功能测试中心").waitForDisplayed();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "get_test_run"),
    ));
    const loadingText = await browser.$("main > div > section").getText();
    assert.ok(loadingText.includes("正在读取所选测试运行"));
    assert.equal(loadingText.includes("管理员运行概览"), false);
    await browser.saveScreenshot(`${artifacts}/detail-loading.png`);
    await browser.execute(() => {
      window.__uiAudit.deferred = [];
      const pending = window.__uiAudit.pending.find((item) => item.command === "get_test_run");
      window.__uiAudit.pending = window.__uiAudit.pending.filter((item) => item !== pending);
      pending?.resolve(Response.json({
        code: "unavailable",
        message: "detail temporarily unavailable",
        retryable: true,
        trace_id: null,
        details: {},
      }, { status: 503 }));
    });
    await $("[role=alert]").waitForDisplayed();
    assert.ok((await $("[role=alert]").getText()).includes("detail temporarily unavailable"));

    await browser.execute((detail) => {
      window.__uiAudit.fixtures.get_test_run = detail;
      window.__uiAudit.deferred = ["list_test_runs"];
    }, runningDetail);
    await $('button[aria-label="刷新"]').click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "list_test_runs"),
    ));
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_test_run").length > 1,
    ));
    await browser.execute((staleRun) => {
      window.__uiAudit.deferred = [];
      const pending = window.__uiAudit.pending.find((item) => item.command === "list_test_runs");
      window.__uiAudit.pending = window.__uiAudit.pending.filter((item) => item !== pending);
      pending?.resolve(Response.json([staleRun]));
    }, queuedRun);
    await browser.waitUntil(async () => browser.execute(() => !document.querySelector("[role=alert]")));
    await browser.waitUntil(async () => browser.execute(() => {
      const row = [...document.querySelectorAll("main > div > aside:last-child li")]
        .find((item) => item.textContent?.includes("项目复制"));
      return row?.textContent?.includes("运行中") ?? false;
    }));
    assert.equal((await browser.$("main > div > section").getText()).includes("测试通过"), false);
    await browser.saveScreenshot(`${artifacts}/detail-recovered.png`);

    const activePollCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_test_run").length,
    );
    await browser.execute(() => window.__uiAudit.deferred.push("get_test_run"));
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "get_test_run").length > count,
    activePollCount));
    const deferredPollCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_test_run").length,
    );
    await browser.pause(950);
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "get_test_run").length,
    ), deferredPollCount, "slow active-run reads must stay single-flight");
    await browser.execute(() => {
      window.__uiAudit.deferred = [];
      const pending = window.__uiAudit.pending.find((item) => item.command === "get_test_run");
      window.__uiAudit.pending = window.__uiAudit.pending.filter((item) => item !== pending);
      pending?.resolve(Response.json(window.__uiAudit.fixtures.get_test_run));
    });

    const category = await $('select[aria-label="分类"]');
    await category.selectByAttribute("value", "system");
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('select[aria-label="分类"]')?.value === "system",
    ));
    await $('//button[contains(normalize-space(.),"管理员运行概览")]').click();
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('main > div > aside:first-child button[aria-pressed="true"]')?.textContent?.includes("管理员运行概览"),
    ));
    await category.selectByAttribute("value", "projects");
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('select[aria-label="分类"]')?.value === "projects",
    ));
    await $('//button[contains(normalize-space(.),"项目复制")]').click();
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('main > div > aside:first-child button[aria-pressed="true"]')?.textContent?.includes("项目复制"),
    ));
    await category.selectByAttribute("value", "system");
    await $('//button[contains(normalize-space(.),"管理员运行概览")]').click();
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('main > div > aside:first-child button[aria-pressed="true"]')?.textContent?.includes("管理员运行概览"),
    ));
    await $("button*=查看正在运行的测试").waitForDisplayed();
    const historyRow = await $("main > div > aside:last-child li button");
    await historyRow.click();
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('select[aria-label="分类"]')?.value === "projects",
    ));
    await browser.saveScreenshot(`${artifacts}/history-selection.png`);
    assert.equal(new URL(await browser.getUrl()).searchParams.get("runId"), runId);

    const projectListRequests = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_all_projects").length,
    );
    await browser.execute((copy) => {
      window.__uiAudit.fixtures.list_all_projects = [
        window.__uiAudit.fixtures.list_all_projects[0], copy,
      ];
    }, copySummary);
    await $('button*=打开项目').click();
    await browser.waitUntil(async () => browser.execute((minimum) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_all_projects").length >= minimum,
    projectListRequests + 1));
    await browser.waitUntil(async () => browser.execute(() => location.pathname === "/video-project/99"));
    await browser.saveScreenshot(`${artifacts}/project-navigation.png`);
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.requests.some((item) =>
      item.command === "get_project" && item.body.projectId === 99,
    )));
    await $("h1=复制后的先导片").waitForDisplayed();
    await browser.saveScreenshot(`${artifacts}/project-rendered.png`);
    assert.deepEqual(await browser.execute(() => window.__uiAudit.unexpected), []);
    assert.deepEqual(await browser.execute(() => window.__uiAudit.errors), []);

    await installUiFixtures(browser, {
      ...fixtures,
      list_test_runs: [run],
      get_test_run: passedDetail,
      list_all_projects: [sourceSummary],
      deferred: [],
    });
    await browser.url(`/admin/test-center?runId=${runId}`);
    await $("h1=功能测试中心").waitForDisplayed();
    await browser.waitUntil(async () => browser.$("button[aria-label=\"删除运行记录\"]").isDisplayed());
    await browser.execute(() => window.__uiAudit.deferred.push("delete_test_run"));
    const detailDelete = await $('button[aria-label="删除运行记录"]');
    await detailDelete.click();
    await $('button*=同时删除测试项目').click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "delete_test_run"),
    ));
    assert.equal(await browser.execute(() =>
      [...document.querySelectorAll('button[aria-label="删除运行记录"]')][0]?.disabled,
    ), true);
    await browser.execute(() => {
      document.querySelector('button[aria-label="删除运行记录"]')?.click();
    });
    assert.equal(await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "delete_test_run").length,
    ), 1, "the detail delete action must not submit twice while in flight");
    await browser.execute(() => {
      window.__uiAudit.deferred = [];
      const pending = window.__uiAudit.pending.find((item) => item.command === "delete_test_run");
      window.__uiAudit.pending = window.__uiAudit.pending.filter((item) => item !== pending);
      pending?.resolve(Response.json({
        code: "unavailable",
        message: "delete temporarily unavailable",
        retryable: true,
        trace_id: null,
        details: {},
      }, { status: 503 }));
    });
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector("[role=alert]")?.textContent?.includes("delete temporarily unavailable") ?? false,
    ));
    await browser.execute(() => { window.__uiAudit.fixtures.delete_test_run = null; });
    await $('button[aria-label="删除运行记录"]').click();
    await $('button*=同时删除测试项目').click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "delete_test_run").length === 2,
    ));
    const deleteRequest = await browser.execute(() =>
      window.__uiAudit.requests.findLast((item) => item.command === "delete_test_run"),
    );
    assert.equal(deleteRequest.body.input.delete_test_project, true);
    await browser.waitUntil(async () => !new URL(await browser.getUrl()).searchParams.has("runId"));
    await browser.waitUntil(async () => browser.execute(() => !document.querySelector("[role=alert]")));
    await $('button*=当前项目').click();
    await browser.saveScreenshot(`${artifacts}/project-cleaned.png`);
    assert.equal(await browser.execute(() =>
      [...document.querySelectorAll("select option")].some((option) => option.textContent?.includes("#99")),
    ), false, "deleting a run-created project must remove it from project choices");
  });

  it("localizes test categories and repository suite descriptions in English", async () => {
    const englishScenarios = [
      {
        id: "admin_overview", version: 1, category: "system", title: "管理员运行概览",
        description: "验证管理员概览统计可以读取。", effect: "read",
        target_modes: ["test_project"], timeout_seconds: 30,
        parameter_schema: { type: "object", properties: {}, additionalProperties: false },
      },
      {
        id: "project_list", version: 1, category: "project", title: "项目列表读取",
        description: "验证管理员可以读取项目列表。", effect: "read",
        target_modes: ["test_project"], timeout_seconds: 30,
        parameter_schema: { type: "object", properties: {}, additionalProperties: false },
      },
      ...scenarios,
    ];
    await installUiFixtures(browser, {
      ...testFixtures,
      list_test_scenarios: englishScenarios,
      list_test_runs: [],
      deferred: [],
    });
    await browser.setWindowSize(1440, 900);
    await browser.url("/admin/test-center?ui-locale=en-US");
    await $("h1=Functional Test Center").waitForDisplayed();

    const category = await $('select[aria-label="Category"]');
    const categoryOptions = [...await category.$$("option")];
    const labels = await Promise.all(categoryOptions.map((option) => option.getText()));
    assert.ok(labels.includes("System"));
    assert.ok(labels.includes("Projects"));
    assert.ok(labels.includes("Core features"));
    assert.ok(labels.includes("Browser features"));
    assert.equal(labels.some((label) => ["system", "project", "核心功能"].includes(label)), false);

    await $("button*=Core unit and integration tests").waitForDisplayed();
    await category.selectByAttribute("value", "核心功能");
    await $("h2=Core unit and integration tests").waitForDisplayed();
    const scenarioCard = await $('main > div > aside:first-child button[aria-pressed="true"]');
    assert.ok((await scenarioCard.getText()).includes("Core features"));
    assert.equal(await browser.execute(() => window.__uiAudit.unexpected.length), 0);
  });
});
