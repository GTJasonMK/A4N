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

async function chooseStage(label) {
  await $('[role="tablist"]').waitForExist();
  const select = await $('[role="combobox"][id$="-select"]');
  if (await select.isDisplayed()) {
    await select.click();
    await $('[role="listbox"]').waitForDisplayed();
    for (const option of await $('[role="listbox"]').$$('[role="option"]')) {
      if ((await option.getText()).includes(label)) {
        await option.click();
        return;
      }
    }
    throw new Error(`Missing stage ${label}`);
  }
  await $(`//button[@role="tab" and contains(.,"${label}")]`).click();
}

const choosePlanning = () => chooseStage("页数与页面规划");
const deleteStageButton = () => $('//button[normalize-space(.)="删除本章本阶段全部产物"]');

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

async function generateManga(target) {
  await callBrowserApiCommand("start_manga_pipeline", { runtime_config_snapshot: fixture.runtime_config_snapshot, input: {
    project_id: target.projectId, chapter_number: 1, image_size: { width: 1024, height: 1024 },
    meta: { start_request_id: randomUUID() },
  } });
  await browser.waitUntil(async () => {
    const status = await callBrowserApiCommand("get_manga_pipeline_status", { input: { project_id: target.projectId, chapter_id: target.chapterId } });
    assert.notEqual(status.pipeline_status, "failed", JSON.stringify(status));
    return status.pipeline_status === "completed";
  }, { timeout: 60_000, interval: 100 });
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
      assert.equal(await deleteStageButton().isEnabled(), false);
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
    await $('button[aria-label="分页产物操作"]').click();
    await clickButton("删除分页产物");
    await waitText(/删除阶段产物/);
    await clickButton("删除", await dialog());
    await waitForNoDialog();
    await browser.waitUntil(async () => {
      const saved = await callBrowserApiCommand("list_manga_pipeline_artifacts", { input: { project_id: failed.projectId, chapter_id: failed.chapterId } });
      return !saved.some((item) => item.stage === "page_slots");
    }, { timeout: 30_000, interval: 100 });
    const cleared = await callBrowserApiCommand("list_manga_page_prompts", { input: { project_id: failed.projectId, chapter_id: failed.chapterId } });
    assert.equal(cleared.length, 2);
    assert.ok(cleared.every((page) => page.pipeline_stage === "content" && page.page_content === null));
    await browser.refresh();
    await choosePlanning();
    await $('//button[normalize-space(.)="生成至页数与页面规划" or normalize-space(.)="继续生成至页数与页面规划"]').click();
    await waitText(/任务已完成/);
    await capture("regenerated-after-delete", 1440);
    await closeDialog();
    checks.push("Deleting pagination keeps the workspace readable after refresh and allows fresh regeneration");
    const scope = { input: { project_id: failed.projectId, chapter_id: failed.chapterId } };
    const beforeDeleteAll = await callBrowserApiCommand("list_manga_pipeline_artifacts", scope);
    const beforeDeletePages = await callBrowserApiCommand("list_manga_page_prompts", scope);
    assert.ok(beforeDeleteAll.some((item) => item.stage === "preprocessing"));
    assert.ok(beforeDeleteAll.some((item) => item.stage === "page_slots"));
    for (const width of [1440, 1024, 390]) {
      await browser.setWindowSize(width, 1000);
      await chooseStage("漫画预处理");
      await deleteStageButton().waitForEnabled();
      await capture("delete-stage-entry", width);
      await deleteStageButton().execute((element) => element.focus());
      await browser.keys("Enter");
      await waitText(/删除本章「漫画预处理」产物/);
      assert.match(await dialog().getText(), /第 1 章.*旧车站/);
      assert.match(await dialog().getText(), /上游产物、原文、改编计划和基础素材会保留/);
      assert.match(await dialog().getText(), /不可撤销/);
      await capture("delete-stage-confirm", width);
      await closeDialog();
      await browser.waitUntil(() => deleteStageButton().isFocused(), { timeout: 3000, interval: 50 });
      assert.deepEqual(await callBrowserApiCommand("list_manga_pipeline_artifacts", scope), beforeDeleteAll);
    }
    checks.push("Chapter-stage deletion is disabled during generation; its stage-specific confirmation works by keyboard, restores focus, and cancellation preserves artifacts at all three widths");
    await deleteStageButton().click();
    await waitText(/删除本章「漫画预处理」产物/);
    await clickButton("删除本章本阶段全部产物", await dialog());
    await waitForNoDialog();
    await browser.waitUntil(async () => (await callBrowserApiCommand("list_manga_pipeline_artifacts", scope)).length === 0, { timeout: 30_000, interval: 100 });
    const afterPreprocessingDelete = await callBrowserApiCommand("list_manga_page_prompts", scope);
    assert.deepEqual(afterPreprocessingDelete.map((page) => page.id), beforeDeletePages.map((page) => page.id));
    assert.ok(afterPreprocessingDelete.every((page) => page.page_content === null && page.generated_image_id === null));
    assert.equal(await deleteStageButton().isEnabled(), false);
    await capture("delete-stage-empty", 390);
    await choosePlanning();
    await browser.refresh();
    await choosePlanning();
    assert.deepEqual(await callBrowserApiCommand("list_manga_page_prompts", scope), afterPreprocessingDelete);
    assert.deepEqual(await callBrowserApiCommand("list_manga_pipeline_artifacts", scope), []);
    await capture("delete-stage-refreshed", 390);
    checks.push("Deleting preprocessing clears its dependent outputs while retaining page identities; the workspace stays readable after refresh");

    await browser.execute((config) => {
      const key = "a4n:local-runtime-settings:v1";
      const settings = JSON.parse(localStorage.getItem(key));
      settings.image = [config];
      settings.default_source.image = "local";
      settings.next_id = Math.max(settings.next_id, config.id + 1);
      localStorage.setItem(key, JSON.stringify(settings));
    }, fixture.runtime_config_snapshot.image);
    const deletion = await freshProject("a4n-virtual-text", 1440);
    await generateManga(deletion);
    await openProject(deletion.projectId);
    const deletionScope = { input: { project_id: deletion.projectId, chapter_id: deletion.chapterId } };
    let originalPages = await callBrowserApiCommand("list_manga_page_prompts", deletionScope);
    assert.equal(originalPages.length, 2);
    assert.ok(originalPages.every((page) => page.generated_image_id));
    for (const [label, stage, removedStages, clearedFields] of [
      ["提示词与成图", "drawing_prompt", ["drawing_prompt", "image"], ["drawing_prompt", "generated_image_id"]],
      ["分镜与参考图", "storyboard", ["storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"], ["page_plan", "reference_discovery", "drawing_prompt", "generated_image_id"]],
      ["页数与页面规划", "page_slots", ["page_slots", "page_content", "page_adaptation", "storyboard", "reference_bindings", "reference_discovery", "drawing_prompt", "image"], ["page_content", "page_adaptation", "page_plan", "reference_discovery", "drawing_prompt", "generated_image_id"]],
    ]) {
      const before = await callBrowserApiCommand("list_manga_pipeline_artifacts", deletionScope);
      const beforePages = await callBrowserApiCommand("list_manga_page_prompts", deletionScope);
      for (const width of [1440, 1024, 390]) {
        await browser.setWindowSize(width, 1000);
        await chooseStage(label);
        await deleteStageButton().waitForEnabled();
        await deleteStageButton().execute((element) => element.focus());
        await browser.keys("Enter");
        await waitText(new RegExp(`删除本章「${label}」产物`));
        assert.match(await dialog().getText(), /本章所有页面，不限当前选中页/);
        assert.match(await dialog().getText(), /上游产物、原文、改编计划和基础素材会保留/);
        await capture(`chapter-${stage}-confirm`, width);
        await closeDialog();
        assert.deepEqual(await callBrowserApiCommand("list_manga_pipeline_artifacts", deletionScope), before);
      }
      await deleteStageButton().click();
      await waitText(new RegExp(`删除本章「${label}」产物`));
      await clickButton("删除本章本阶段全部产物", await dialog());
      await waitForNoDialog();
      await browser.waitUntil(async () => (await callBrowserApiCommand("list_manga_pipeline_artifacts", deletionScope)).every((artifact) => !removedStages.includes(artifact.stage)));
      const retained = await callBrowserApiCommand("list_manga_pipeline_artifacts", deletionScope);
      assert.deepEqual(retained, before.filter((artifact) => !removedStages.includes(artifact.stage)));
      const afterPages = await callBrowserApiCommand("list_manga_page_prompts", deletionScope);
      assert.deepEqual(afterPages.map((page) => page.id), originalPages.map((page) => page.id));
      for (let index = 0; index < afterPages.length; index += 1) {
        for (const field of ["page_content", "page_adaptation", "page_plan", "reference_discovery", "drawing_prompt", "generated_image_id"]) {
          if (clearedFields.includes(field)) assert.equal(afterPages[index][field], null, `${label}: ${field}`);
          else assert.deepEqual(afterPages[index][field], beforePages[index][field], `${label}: upstream ${field}`);
        }
      }
      await browser.refresh();
      await chooseStage(label);
      await deleteStageButton().waitForDisplayed();
      assert.equal(await deleteStageButton().isEnabled(), false);
      assert.deepEqual(await callBrowserApiCommand("list_manga_pipeline_artifacts", deletionScope), retained);
      await capture(`chapter-${stage}-deleted`, 390);
      checks.push(`${label}: chapter-wide deletion clears both pages from the current stage, preserves upstream artifacts and page identities, survives refresh, and cancellation preserves data at three widths`);
    }
    for (const label of ["基础素材", "改编计划"]) {
      await chooseStage(label);
      assert.equal(await deleteStageButton().isExisting(), false);
    }
    checks.push("Materials and adaptation use their own controls and do not expose pipeline chapter-stage deletion");
    await generateManga(deletion);
    await openProject(deletion.projectId);
    originalPages = await callBrowserApiCommand("list_manga_page_prompts", deletionScope);
    const promptSection = () => $('//section[./div/h3[normalize-space(.)="整页提示词"]]');
    const selectedPage = () => $('nav[aria-label="漫画页列表"] button[aria-current="true"]');
    for (const width of [1440, 1024, 390]) {
      await browser.setWindowSize(width, 1000);
      await chooseStage("提示词与成图");
      await promptSection().$('button[aria-haspopup="menu"]').waitForEnabled();
      await promptSection().$('button[aria-haspopup="menu"]').click();
      await clickButton("删除 · 整页提示词");
      await waitText(/删除阶段产物/);
      assert.match(await dialog().getText(), /整页成图/);
      await capture("drawing-delete-confirm", width);
      await closeDialog();
      assert.deepEqual(await callBrowserApiCommand("list_manga_page_prompts", deletionScope), originalPages);
    }
    await promptSection().$('button[aria-haspopup="menu"]').click();
    await clickButton("删除 · 整页提示词");
    await waitText(/删除阶段产物/);
    await clickButton("删除", await dialog());
    await waitForNoDialog();
    await browser.waitUntil(async () => (await promptSection().getText()).includes("整页绘图提示词尚未生成"), { timeout: 30_000 });
    assert.match(await selectedPage().getText(), /第 1 页/);
    const afterDrawing = await callBrowserApiCommand("list_manga_page_prompts", deletionScope);
    assert.deepEqual(afterDrawing.map((page) => page.id), originalPages.map((page) => page.id));
    assert.ok(afterDrawing[0].page_plan);
    assert.equal(afterDrawing[0].drawing_prompt, null);
    assert.equal(afterDrawing[0].generated_image_id, null);
    assert.deepEqual(afterDrawing[1], originalPages[1]);
    await capture("drawing-deleted-page-retained", 390);
    checks.push("Drawing deletion cascades to the image, retains both page identities and selection, refreshes immediately, and cancellation preserves data at three widths");

    await browser.setWindowSize(1440, 1000);
    await chooseStage("分镜与参考图");
    const storyboardSection = () => $('//section[./div/h3[normalize-space(.)="单页分镜"]]');
    await clickButton("删除", await storyboardSection());
    await waitText(/删除阶段产物/);
    assert.match(await dialog().getText(), /参考图/);
    await clickButton("删除", await dialog());
    await waitForNoDialog();
    await browser.waitUntil(async () => (await storyboardSection().getText()).includes("分镜尚未生成"), { timeout: 30_000 });
    const afterStoryboard = await callBrowserApiCommand("list_manga_page_prompts", deletionScope);
    assert.equal(afterStoryboard[0].page_plan, null);
    assert.equal(afterStoryboard[0].reference_discovery, null);
    assert.deepEqual(afterStoryboard[1], originalPages[1]);
    await capture("storyboard-deleted", 1440);
    checks.push("Storyboard deletion refreshes its dependent reference discovery and preserves the other page without a reload");

    await generateManga(deletion);
    await openProject(deletion.projectId);
    await chooseStage("改编计划");
    for (const width of [1440, 1024, 390]) {
      await browser.setWindowSize(width, 1000);
      await $('//button[normalize-space(.)="删除"]').waitForEnabled();
      await $('//button[normalize-space(.)="删除"]').click();
      await waitText(/漫画章节阶段产物/);
      assert.match(await dialog().getText(), /页数与/);
      assert.match(await dialog().getText(), /第 1 页/);
      await capture("adaptation-delete-cascade", width);
      await closeDialog();
    }
    await $('//button[normalize-space(.)="删除"]').click();
    await waitText(/漫画章节阶段产物/);
    await clickButton("删除", await dialog());
    await waitForNoDialog();
    await browser.waitUntil(async () => (await callBrowserApiCommand("list_manga_page_prompts", deletionScope)).length === 0, { timeout: 30_000 });
    const remaining = await callBrowserApiCommand("list_manga_pipeline_artifacts", deletionScope);
    assert.deepEqual(remaining.map((artifact) => artifact.stage), ["preprocessing"]);
    await choosePlanning();
    await browser.waitUntil(async () => (await $("body").getText()).includes("生成至页数与页面规划"), { timeout: 30_000 });
    await capture("adaptation-deleted-workspace-refreshed", 390);
    await browser.refresh();
    await choosePlanning();
    assert.deepEqual(await callBrowserApiCommand("list_manga_page_prompts", deletionScope), []);
    checks.push("Adaptation confirmation shows chapter pagination and both pages at all widths; deleting it clears the workspace and pagination while preserving preprocessing after refresh");
    complete = true;
  });
});
