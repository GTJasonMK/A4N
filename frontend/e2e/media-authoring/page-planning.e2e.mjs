import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { login, clickButton, buttonMatching, waitForNoDialog } from "../full-project/user-flow.mjs";
import { callBrowserApiCommand } from "../full-project/webApi.mjs";

const fixture = JSON.parse(readFileSync(process.env.A4N_MEDIA_AUTHORING_CLI_REPORT, "utf8"));
const artifacts = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
const checks = [];
let complete = false;
const dialog = () => $('.a4n-modal-layer:not([data-modal-state="exiting"]) [role="dialog"]');

async function setModel(model) {
  await browser.execute((name) => {
    const key = "a4n:local-runtime-settings:v1";
    const settings = JSON.parse(localStorage.getItem(key));
    const selected = settings.llm.find((item) => item.is_default);
    if (!selected) throw new Error("Missing isolated model config");
    Object.assign(selected, { model: name, max_output_tokens: 20_000, max_context_tokens: 128_000, max_concurrent: 2 });
    localStorage.setItem(key, JSON.stringify(settings));
  }, model);
}

async function choosePlanning() {
  await $('[role="tablist"]').waitForExist();
  const select = await $('[role="combobox"][id$="-select"]');
  if (await select.isDisplayed()) {
    await select.click();
    await $('[role="listbox"]').waitForDisplayed();
    for (const option of await $('[role="listbox"]').$$('[role="option"]')) {
      if ((await option.getText()).includes("页数与页面规划")) {
        await option.click();
        return;
      }
    }
    throw new Error("Missing page planning stage");
  }
  await $('//button[@role="tab" and contains(.,"页数与页面规划")]').click();
}

async function openProject(id) {
  await browser.url(`/comic-project/${id}`);
  await choosePlanning();
}

async function freshProject(model, width) {
  await setModel("a4n-virtual-text");
  const project = await callBrowserApiCommand("create_derived_project", { input: {
    root_project_id: fixture.ui_root_id, derived_kind: "comic", title: `分页 ${model} ${width}`,
    description: "isolated page planning browser acceptance", start_chapter: 1, end_chapter: 1,
  } });
  const workspace = await callBrowserApiCommand("get_media_authoring_workspace", { input: { project_id: project.id, chapter_id: null } });
  const chapterId = workspace.chapters[0].source.source_chapter_id;
  await callBrowserApiCommand("start_manga_pipeline", { runtime_config_snapshot: fixture.runtime_config_snapshot, input: {
    project_id: project.id, chapter_number: 1, run_stage: "preprocessing", meta: { start_request_id: randomUUID() },
  } });
  await browser.waitUntil(async () => {
    const status = await callBrowserApiCommand("get_manga_pipeline_status", { input: { project_id: project.id, chapter_id: chapterId } });
    assert.notEqual(status.pipeline_status, "failed", JSON.stringify(status));
    return status.pipeline_status === "completed";
  }, { timeout: 60_000, interval: 100 });
  const record = await callBrowserApiCommand("generate_media_adaptation_plan", { runtime_config_snapshot: fixture.runtime_config_snapshot, input: {
    project_id: project.id, chapter_id: chapterId, task_id: randomUUID(), instruction: "保留原作因果，以漫画独立改编",
    video_reference: null, clear_video_reference: false, record_id: null, expected_revision: null, resume_from_checkpoint: true,
  } });
  await callBrowserApiCommand("adopt_media_authoring_document", { input: { project_id: project.id, record_id: record.id, expected_revision: record.revision } });
  await setModel(model);
  await browser.setWindowSize(width, 1000);
  await openProject(project.id);
  return { projectId: project.id, chapterId };
}

async function waitText(pattern) {
  await browser.waitUntil(async () => await dialog().isDisplayed() && pattern.test(await dialog().getText()), {
    timeout: 60_000, interval: 100, timeoutMsg: `Page planning dialog missing ${pattern}`,
  });
}

async function closeDialog() {
  await browser.keys("Escape");
  await waitForNoDialog();
}

async function capture(name, width) {
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  await browser.saveScreenshot(`${artifacts}/page-planning-${name}-${width}.png`);
  checks.push(`${name} ${width}px: no horizontal overflow`);
}

describe("Per-page manga planning against the real Host", () => {
  before(async () => {
    await login();
    await browser.url("/settings");
    await clickButton("LLM 服务");
    await clickButton("新增 LLM 配置");
    for (const [label, value] of [
      ["配置名称", "分页隔离验收"], ["模型", "a4n-virtual-text"],
      ["接口地址", process.env.A4N_BROWSER_E2E_PROVIDER_URL], ["访问密钥", "a4n-e2e-provider-key"],
    ]) await $(`//label[contains(.,"${label}")]//input`).setValue(value);
    await clickButton(/创建|保存/);
    await buttonMatching("分页隔离验收");
  });

  after(() => writeFileSync(`${artifacts}/page-planning-report.json`, JSON.stringify({ success: complete, checks }, null, 2)));

  it("shows real dependencies, per-page progress, cancellation, resume and refresh", async () => {
    for (const width of [1440, 1024, 390]) {
      const target = await freshProject("a4n-page-planning-slow", width);
      const preview = await buttonMatching(/查看生成至.*页数与页面规划.*请求/);
      await preview.execute((element) => element.focus());
      await browser.keys("Enter");
      await waitText(/轻量页纲/);
      assert.match(await dialog().getText(), /20[,，]?000/);
      await clickButton("第 1 页剧情内容", await dialog());
      await waitText(/等待轻量页纲/);
      assert.doesNotMatch(await dialog().getText(), /暂无预览数据/);
      await capture("waiting", width);
      await closeDialog();
      await browser.waitUntil(() => preview.isFocused(), { timeout: 3000, interval: 50 });
      await $('//button[normalize-space(.)="生成至页数与页面规划"]').click();
      await waitText(/第 [12] 页剧情内容：生成中/);
      await capture("running", width);
      if (width === 390) {
        await clickButton("取消流水线", await dialog());
        await waitText(/已取消|已停止/);
        await closeDialog();
        await buttonMatching("继续生成至页数与页面规划");
        const saved = await callBrowserApiCommand("list_manga_pipeline_artifacts", { input: { project_id: target.projectId, chapter_id: target.chapterId } });
        assert.equal(saved.filter((item) => item.stage === "page_slots").length, 0);
        checks.push("Cancellation leaves no partial pagination artifact and exposes checkpoint resume");
      } else if (width === 1024) {
        await browser.refresh();
        await choosePlanning();
        await browser.waitUntil(async () => {
          const saved = await callBrowserApiCommand("list_manga_pipeline_artifacts", { input: { project_id: target.projectId, chapter_id: target.chapterId } });
          return saved.some((item) => item.stage === "page_slots");
        }, { timeout: 60_000, interval: 100 });
        await $('button[aria-label="分页产物操作"]').waitForDisplayed({ timeout: 30_000 });
        await capture("refreshed", width);
        checks.push("Refresh recovers completed pagination and opens the existing editor");
      } else {
        await waitText(/任务已完成/);
        const text = await dialog().getText();
        assert.match(text, /页面内容已完成 2\/2 页/);
        assert.match(text, /Token/);
        assert.equal(await dialog().$('[role="progressbar"]').getAttribute("aria-valuenow"), "100");
        await capture("completed", width);
        await closeDialog();
        await clickButton("分页产物操作");
        await clickButton("编辑分页与每页内容");
        await browser.waitUntil(async () => (await $("body").getText()).includes("先生成轻量页纲"), { timeout: 5000 });
        await capture("editor", width);
        await clickButton("取消");
        checks.push("All validated pages are editable only after the complete result is saved");
      }
    }
    const failed = await freshProject("a4n-page-planning-fail-content", 1440);
    await $('//button[normalize-space(.)="生成至页数与页面规划"]').click();
    await waitText(/输出纠正仍未完成/);
    await waitText(/任务失败/);
    assert.match(await dialog().getText(), /第 2 页剧情内容/);
    await capture("failed", 1440);
    await closeDialog();
    await setModel("a4n-virtual-text");
    await openProject(failed.projectId);
    await $('//button[contains(.,"查看生成至") and contains(.,"页数与页面规划")]').click();
    await waitText(/轻量页纲/);
    assert.match(await dialog().getText(), /复用/);
    await capture("resume-preview", 1440);
    await closeDialog();
    await $('//button[normalize-space(.)="继续生成至页数与页面规划"]').click();
    await waitText(/任务已完成/);
    assert.match(await dialog().getText(), /已复用检查点/);
    await closeDialog();
    checks.push("Failure identifies page 2; resume previews and reuses the outline and successful page");
    complete = true;
  });
});
