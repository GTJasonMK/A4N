import assert from "node:assert/strict";

const projectId = Number(process.env.A4N_NONLINEAR_EXISTING_PROJECT_ID);
const llmApiKey = process.env.A4N_NONLINEAR_LLM_API_KEY;

const buttonByText = async (text) => {
  const button = await $(`//button[contains(normalize-space(.), "${text}")]`);
  await button.waitForClickable();
  return button;
};

const openTab = async (accessibleLabel) => {
  const tab = await $(`[role=tab][aria-label="${accessibleLabel}"]`);
  await tab.waitForClickable({ timeout: 30_000 });
  await tab.click();
  await browser.waitUntil(() => tab.getAttribute("aria-selected").then((value) => value === "true"));
};

const closeSuccessfulProgress = async (successText, timeout = 480_000) => {
  const dialog = await $("[role=dialog]");
  await dialog.waitForDisplayed({ timeout: 30_000 });
  await browser.waitUntil(
    () => dialog.getText().then((text) => text.includes(successText)),
    { timeout, interval: 2_000, timeoutMsg: `任务未完成：${successText}` },
  );
  const close = await dialog.$("button*=关闭");
  await close.waitForClickable();
  await close.click();
  await dialog.waitForDisplayed({ reverse: true, timeout: 30_000 });
};

const openMaterialSummary = async () => {
  await openTab("内容导入与项目资料归纳");
  const details = await $("//details[.//span[normalize-space()='项目资料归纳']]");
  await details.waitForDisplayed({ timeout: 30_000 });
  if (!(await details.getAttribute("open"))) await details.$("summary").click();
  return details;
};

describe("恢复稀疏章节资料同步", () => {
  before(async () => {
    assert.ok(llmApiKey, "真实模型验收缺少临时 LLM API Key");
    await browser.execute((apiKey) => {
      localStorage.setItem(
        "a4n:local-runtime-settings:v1",
        JSON.stringify({
          schema_version: 1,
          next_id: 2,
          llm: [],
          system_llm_api_key_encrypted: apiKey,
          system_image_api_key_encrypted: null,
          system_video_api_key_encrypted: null,
          embedding: [
            {
              id: 1,
              name: "基础本地哈希向量",
              backend: "local_hash",
              base_url: null,
              api_key_encrypted: null,
              has_api_key: false,
              model: "local-sha256-hash",
              dimension: 384,
              batch_size: 100,
              local_model_path: null,
              is_default: true,
            },
          ],
          image: [],
          video: [],
          default_source: { llm: "system", image: "system", video: "system" },
          llm_max_tokens: {
            connection_test: 8,
            short: 1024,
            standard: 2048,
            json_standard: 4096,
            long: 8192,
            extra_long: 8192,
            chapter_generation: 8192,
            chapter_analysis: 4096,
            outline: 8192,
            blueprint: 8192,
            character_discovery: 8192,
            character_sync: 8192,
            media_prompt: 4096,
          },
        }),
      );
    }, llmApiKey);
    await browser.refresh();
  });

  it("第 7 章完成后在真实写作台展示全部资料同步结果", async () => {
    assert.ok(Number.isSafeInteger(projectId) && projectId > 0);
    await browser.url(`/write/${projectId}?chapter=7`);
    const analysisTab = await $(
      "//nav[@aria-label='章节工作区']//button[normalize-space()='分析']",
    );
    await analysisTab.waitForClickable({ timeout: 30_000 });
    await analysisTab.click();

    const panel = await $("section[aria-label='本章资料同步状态']");
    await panel.waitForDisplayed({ timeout: 30_000 });
    const statuses = await panel.$$("dd").map((item) => item.getText());
    assert.deepEqual(statuses, Array(5).fill("已完成"));
    assert.equal(await $("button*=继续同步").isExisting(), false);

    const details = await panel.$("details");
    await details.$("summary").click();
    const detailText = await details.getText();
    assert.match(detailText, /同步角色状态「林雾」/);
    assert.match(detailText, /同步角色状态「周砚」/);
    assert.doesNotMatch(detailText, /角色档案同步范围不完整/);
  }).timeout(90_000);

  it("用户主动归纳分部后确认写入正式资料，并保留已有章纲", async () => {
    await browser.url(`/novel/${projectId}`);
    const details = await openMaterialSummary();
    const target = await details.$("select");
    await target.selectByAttribute("value", "part_outline");

    await browser.pause(2_000);
    if (!(await details.getText()).includes("分部大纲候选（尚未写入正式资料）")) {
      await (await buttonByText("生成分部大纲候选")).click();
      await closeSuccessfulProgress("分部大纲候选已生成，尚未写入正式资料");
    }

    await browser.waitUntil(
      () => details.getText().then((text) => text.includes("分部大纲候选（尚未写入正式资料）")),
      { timeout: 30_000, timeoutMsg: "分部归纳结果未停留在待确认状态" },
    );
    await (await buttonByText("确认更新分部大纲")).click();
    const confirm = await $("[role=dialog]");
    await confirm.waitForDisplayed();
    assert.match(await confirm.getText(), /候选未覆盖的分部会保留/);
    const apply = await confirm.$("//button[normalize-space()='更新分部大纲']");
    await apply.waitForClickable();
    await apply.click();
    await confirm.waitForDisplayed({ reverse: true, timeout: 60_000 });
    await browser.waitUntil(
      () => details.getText().then((text) => text.includes("分部大纲候选已应用")),
      { timeout: 60_000, timeoutMsg: "分部归纳结果未显示为已确认" },
    );

    await openTab("章节大纲");
    const outlineText = await $("body").getText();
    assert.match(outlineText, /第 1 章/);
    assert.match(outlineText, /第 7 章/);
  }).timeout(600_000);

  it("用户主动更新蓝图后直接形成正式版本，不出现候选蓝图", async () => {
    await browser.url(`/novel/${projectId}`);
    await openTab("项目概览");
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("故事摘要")),
      { timeout: 30_000 },
    );
    const opened = await browser.execute(() => {
      const heading = [...document.querySelectorAll("h2")].find(
        (item) => item.textContent?.trim() === "故事摘要",
      );
      const button = heading?.closest("header")?.querySelector("button[title='更多操作']");
      if (!(button instanceof HTMLElement)) return false;
      button.click();
      return true;
    });
    assert.equal(opened, true, "故事摘要缺少蓝图更新入口");

    await (await buttonByText("依据当前资料更新蓝图")).click();
    await closeSuccessfulProgress("蓝图已归纳并写入正式资料");

    const pageText = await $("body").getText();
    assert.doesNotMatch(pageText, /暂定蓝图|候选蓝图|蓝图候选/);
    assert.equal(await $("button*=确认更新蓝图").isExisting(), false);
  }).timeout(600_000);
});
