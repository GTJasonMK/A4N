import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { login, clickButton, buttonMatching, waitForNoDialog } from "../full-project/user-flow.mjs";
import { callBrowserApiCommand } from "../full-project/webApi.mjs";

const fixture = JSON.parse(readFileSync(process.env.A4N_MEDIA_AUTHORING_CLI_REPORT, "utf8"));
const artifacts = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
const checks = [];
let complete = false;
assert.ok(fixture.success && fixture.ui_root_id, "Run isolated media-authoring CLI acceptance first");
const dialog = () => $('.a4n-modal-layer:not([data-modal-state="exiting"]) [role="dialog"]');

async function choosePreprocessing() {
  await $('[role="tablist"]').waitForExist();
  const select = await $('[role="combobox"][id$="-select"]');
  if (await select.isDisplayed()) {
    await select.click();
    await $('[role="listbox"]').waitForDisplayed();
    for (const option of await $('[role="listbox"]').$$('[role="option"]')) {
      if ((await option.getText()).includes("漫画预处理")) {
        await option.click();
        return;
      }
    }
    throw new Error("Missing preprocessing stage");
  }
  await $('//button[@role="tab" and contains(.,"漫画预处理")]').click();
}

async function setModel(model) {
  // Only seed the isolated browser's provider config; all page requests use the production client.
  await browser.execute((name) => {
    const key = "a4n:local-runtime-settings:v1";
    const settings = JSON.parse(localStorage.getItem(key));
    const selected = settings.llm.find((item) => item.is_default);
    if (!selected) throw new Error("Missing configured test model");
    selected.model = name;
    selected.max_output_tokens = 20_000;
    selected.max_context_tokens = 128_000;
    selected.max_concurrent = 2;
    localStorage.setItem(key, JSON.stringify(settings));
  }, model);
}

async function openProject(id) {
  await browser.url(`/comic-project/${id}`);
  await choosePreprocessing();
}

async function freshProject(model, width) {
  await setModel(model);
  const project = await callBrowserApiCommand("create_derived_project", { input: {
    root_project_id: fixture.ui_root_id, derived_kind: "comic", title: `预处理 ${model} ${width}`,
    description: "isolated preprocessing browser acceptance", start_chapter: 1, end_chapter: 1,
  } });
  await browser.setWindowSize(width, 1000);
  await openProject(project.id);
  return project.id;
}

async function capture(name, width) {
  const overflow = await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.equal(overflow, 0, `${name} at ${width}px overflows`);
  await browser.saveScreenshot(`${artifacts}/preprocessing-${name}-${width}.png`);
  checks.push(`${name} ${width}px: no horizontal overflow`);
}

async function waitDialogText(pattern) {
  await browser.waitUntil(async () => await dialog().isDisplayed() && pattern.test(await dialog().getText()), {
    timeout: 60_000, interval: 100, timeoutMsg: `Progress missing ${pattern}`,
  });
}

async function closeDialog() {
  await browser.keys("Escape");
  await waitForNoDialog();
}

async function terminalProgress() {
  await waitDialogText(/任务已完成/);
  assert.equal(await dialog().$('[role="progressbar"]').getAttribute("aria-valuenow"), "100");
  const text = await dialog().getText();
  for (const title of ["原文事件与人物", "对白与旁白", "场景与道具", "章节综合"]) assert.ok(text.includes(title), title);
  assert.match(text, /输出 \d+ 字符/);
  assert.match(text, /Token/);
  return text;
}

describe("Four-step manga preprocessing against the real Host", () => {
  before(async () => {
    await login();
    await browser.url("/settings");
    await clickButton("LLM 服务");
    await clickButton("新增 LLM 配置");
    for (const [label, value] of [
      ["配置名称", "预处理隔离验收"], ["模型", "a4n-preprocessing-slow"],
      ["接口地址", process.env.A4N_BROWSER_E2E_PROVIDER_URL], ["访问密钥", "a4n-e2e-provider-key"],
    ]) {
      await $(`//label[contains(.,"${label}")]//input`).setValue(value);
    }
    await clickButton(/创建|保存/);
    await buttonMatching("预处理隔离验收");
  });

  after(() => {
    writeFileSync(`${artifacts}/preprocessing-report.json`, JSON.stringify({ success: complete, checks }, null, 2));
  });

  it("shows dependencies, progress, correction, resume, cancellation and refresh recovery", async () => {
    for (const width of [1440, 1024, 390]) {
      const projectId = await freshProject("a4n-preprocessing-slow", width);
      const preview = await buttonMatching("查看生成请求");
      await preview.execute((element) => element.focus());
      await browser.keys("Enter");
      await waitDialogText(/原文事件与人物/);
      assert.equal(await browser.execute(() => document.activeElement?.closest('[role="dialog"]') !== null), true);
      assert.match(await dialog().getText(), /20[,，]?000/);
      await clickButton("对白与旁白", await dialog());
      await waitDialogText(/等待上游资料/);
      assert.match(await dialog().getText(), /等待上游步骤/);
      assert.doesNotMatch(await dialog().getText(), /暂无预览数据|此步骤没有独立提示词文本/);
      await capture("waiting", width);
      await closeDialog();
      await browser.waitUntil(() => preview.isFocused(), {
        timeout: 3000, interval: 50, timeoutMsg: "Escape must restore focus after the dialog exit animation",
      });
      await clickButton("生成漫画预处理");
      await waitDialogText(/对白与旁白.*正在生成|场景与道具.*正在生成/);
      await capture("running", width);
      if (width === 390) {
        await clickButton("取消流水线", await dialog());
        await waitDialogText(/已取消|已停止/);
        await capture("cancelled", width);
        await closeDialog();
        await buttonMatching(/继续生成至漫画预处理/);
        checks.push("Cancel reaches a terminal state and exposes checkpoint resume");
      } else if (width === 1024) {
        await browser.refresh();
        await choosePreprocessing();
        await browser.waitUntil(async () => (await $("body").getText()).includes("章节叙事重点"), {
          timeout: 60_000, interval: 100, timeoutMsg: "Refresh did not recover the saved preprocessing artifact",
        });
        await capture("refreshed", width);
        checks.push("Reload during parallel extraction recovers the completed artifact");
      } else {
        await terminalProgress();
        await capture("completed", width);
        await closeDialog();
        checks.push("All four validated steps, output counts and usage remain visible after completion");
      }
      const workspace = await callBrowserApiCommand("get_media_authoring_workspace", { input: { project_id: projectId, chapter_id: null } });
      const sourceChapterId = workspace.chapters[0].source.source_chapter_id;
      const saved = await callBrowserApiCommand("list_manga_pipeline_artifacts", { input: { project_id: projectId, chapter_id: sourceChapterId } });
      assert.equal(saved.filter((item) => item.stage === "preprocessing").length, width === 390 ? 0 : 1);
    }

    const failedProject = await freshProject("a4n-preprocessing-fail-speech", 1440);
    await clickButton("生成漫画预处理");
    await waitDialogText(/输出纠正仍未完成/);
    const failure = await dialog().getText();
    assert.ok(failure.includes("对白与旁白失败"));
    assert.ok(failure.includes("唯一一次纠正"));
    await capture("failed", 1440);
    await closeDialog();
    await buttonMatching("继续生成至漫画预处理");
    await setModel("a4n-virtual-text");
    await openProject(failedProject);
    await clickButton("查看继续生成请求");
    await waitDialogText(/场景与道具/);
    assert.match(await dialog().getText(), /复用/);
    await capture("resume-preview", 1440);
    await closeDialog();
    await clickButton("继续生成至漫画预处理");
    assert.match(await terminalProgress(), /已复用检查点/);
    await closeDialog();
    checks.push("Failure exposes the actual contract error; resume previews and reuses successful checkpoints");

    await freshProject("a4n-preprocessing-recover-speech", 1440);
    await clickButton("生成漫画预处理");
    assert.match(await terminalProgress(), /唯一一次纠正/);
    await capture("corrected", 1440);
    await closeDialog();
    checks.push("One corrective request completes without hiding the correction event");
    complete = true;
  });
});
