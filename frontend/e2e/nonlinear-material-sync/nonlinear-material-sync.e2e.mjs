import assert from "node:assert/strict";
import {
  clickButton, currentDialog, fieldByLabel as dialogField, login,
  tab as openTab, closeDialog, waitAndCloseProgress,
  waitForBodyText, waitForNoDialog,
} from "../full-project/user-flow.mjs";

let freeProjectId = null;
const llmApiKey = process.env.A4N_NONLINEAR_LLM_API_KEY;
const llmProvider = process.env.A4N_NONLINEAR_LLM_PROVIDER;
const llmBaseUrl = process.env.A4N_NONLINEAR_LLM_BASE_URL;
const llmModel = process.env.A4N_NONLINEAR_LLM_MODEL;

const createProject = async (mode, expectedRoute) => {
  await clickButton("创建项目");
  await clickButton(mode, await currentDialog());
  await browser.waitUntil(
    () => browser.getUrl().then((url) => url.includes(expectedRoute)),
    { timeoutMsg: `${mode} 未进入 ${expectedRoute}` },
  );
  const url = await browser.getUrl();
  const projectId = Number(url.match(/\/(?:inspiration|novel)\/(\d+)/)?.[1]);
  assert.ok(Number.isSafeInteger(projectId) && projectId > 0, `${mode} 缺少项目 ID`);
  return projectId;
};

const openWritingAnalysis = async () => {
  const analysisTab = await $(
    "//nav[@aria-label='章节工作区']//button[normalize-space()='分析']",
  );
  await analysisTab.waitForClickable({ timeout: 30_000 });
  if ((await analysisTab.getAttribute("aria-selected")) !== "true") {
    await analysisTab.click();
  }
  const panel = await $("section[aria-label='本章资料同步状态']");
  await panel.waitForDisplayed({ timeout: 30_000 });
  return panel;
};

const waitForMaterialSync = async ({ all = false, timeoutMessage }) => {
  await openWritingAnalysis();
  let statuses = [];
  await browser.waitUntil(
    async () => {
      const panel = await $("section[aria-label='本章资料同步状态']");
      statuses = (await panel.$$("dd").map((row) => row.getText())).map((text) => text.trim());
      return statuses.includes("同步失败")
        || (all
          ? statuses.length === 5 && statuses.every((status) => status === "已完成")
          : statuses[0] === "已完成");
    },
    { timeout: 180_000, interval: 2_000, timeoutMsg: timeoutMessage },
  );
  assert.equal(statuses.includes("同步失败"), false, statuses.join(" / "));
  assert.equal(await $("button*=继续同步").isExisting(), false, "章节分析仍有未完成步骤");
  assert.equal(await $("button*=重试资料同步").isExisting(), false, "章节正文资料同步失败");
};

const generateChapterBody = async (projectId, timeoutMessage) => {
  await browser.url(`/write/${projectId}?chapter=1`);
  const assistant = await $(
    "//div[@role='group' and @aria-label='写作辅助面板']//button[contains(., '写作助手')]",
  );
  await assistant.waitForClickable({ timeout: 30_000 });
  await assistant.click();
  const lengthFields = await $("//aside[@aria-label='写作助手']//details[.//span[normalize-space()='正文长度']]");
  if (!(await lengthFields.getAttribute("open"))) await lengthFields.$("summary").click();
  const wordCountInputs = await lengthFields.$$("input[type=number]");
  assert.equal(wordCountInputs.length, 2, "写作助手缺少正文长度范围输入");
  await wordCountInputs[0].setValue("800");
  await wordCountInputs[1].setValue("1200");
  await clickButton("开始生成", await $("aside[aria-label='写作助手']"));
  await clickButton(/^生成\s/, await currentDialog());
  await waitForNoDialog();
  await browser.waitUntil(
    async () => !(await $(".a4n-stream-control[aria-busy='true']").isExisting())
      && (await $("textarea[aria-label='章节正文']").getValue()).trim().length >= 500,
    { timeout: 480_000, interval: 2_000, timeoutMsg: timeoutMessage },
  );
  await browser.refresh();
  const editor = await $("textarea[aria-label='章节正文']");
  await editor.waitForDisplayed();
  assert.ok((await editor.getValue()).trim().length >= 500, `${timeoutMessage}：刷新后正文丢失`);
  await waitForMaterialSync({ timeoutMessage: `${timeoutMessage}：正文索引未完成` });
};

const openMaterialSummary = async () => {
  await openTab("内容导入与项目资料归纳");
  const details = await $("//details[.//span[normalize-space()='项目资料归纳']]");
  await details.waitForDisplayed();
  if (!(await details.getAttribute("open"))) await details.$("summary").click();
  return details;
};

describe("非线性创作资料同步", () => {
  before(async () => {
    assert.ok(llmApiKey, "模型验收缺少 A4N_NONLINEAR_LLM_API_KEY");
    const customLlm = [llmProvider, llmBaseUrl, llmModel].filter(Boolean);
    assert.ok(customLlm.length === 0 || customLlm.length === 3, "自定义模型必须同时提供 provider、base URL 和 model");
    await login();
    const localLlm = llmProvider && llmBaseUrl && llmModel
      ? [{
          id: 2,
          name: "非线性创作真实验收",
          provider: llmProvider,
          base_url: llmBaseUrl,
          api_key_encrypted: llmApiKey,
          has_api_key: true,
          model: llmModel,
          max_concurrent: 1,
          timeout_secs: 300,
          max_context_tokens: 1_024_000,
          max_output_tokens: 52_288,
          extra_body_json: null,
          is_default: true,
        }]
      : [];
    await browser.execute((apiKey, llm) => {
      localStorage.setItem(
        "a4n:local-runtime-settings:v1",
        JSON.stringify({
          schema_version: 1,
          next_id: llm.length > 0 ? 3 : 2,
          llm,
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
          default_source: {
            llm: llm.length > 0 ? "local" : "system",
            image: "system",
            video: "system",
          },
        }),
      );
    }, llmApiKey, localLlm);
    await browser.refresh();
  });

  it("创建模式只保留三个入口，两个文本模式只改变首次落点", async () => {
    await browser.url("/");
    await clickButton("创建项目");
    const modeDialog = await currentDialog();
    const modeLabels = await browser.execute(() =>
      [...document.querySelectorAll("[role=dialog] button")]
        .map((button) => button.textContent?.trim() ?? "")
        .filter((text) =>
          ["AI 辅助创作", "自由创作", "独立画布项目"].some((label) =>
            text.includes(label),
          ),
        ),
    );
    const dialogText = await modeDialog.getText();
    assert.equal(dialogText.includes("AI 辅助创作"), true);
    assert.equal(dialogText.includes("自由创作"), true);
    assert.equal(dialogText.includes("独立画布项目"), true);
    assert.equal(dialogText.includes("小说项目"), false);
    assert.equal(dialogText.includes("剧本项目"), false);
    assert.equal(modeLabels.length, 3);

    await browser.keys("Escape");
    await waitForNoDialog();
    const aiProjectId = await createProject("AI 辅助创作", "/inspiration/");

    await browser.url("/");
    freeProjectId = await createProject("自由创作", "/novel/");
    assert.notEqual(aiProjectId, freeProjectId);
  });

  it("AI 辅助创作可以从完整想法生成正式蓝图、大纲和正文", async () => {
    await browser.url("/");
    const projectId = await createProject("AI 辅助创作", "/inspiration/");
    const idea = [
      "作品暂名《潮痕回声》，核心火花是海滨城每逢退潮就会出现一段只属于失踪者的街道。",
      "类型是都市奇幻悬疑，基调克制、潮湿而温暖；文风采用第三人称限知、简洁有画面感的现实主义笔触。",
      "主角沈汐是能听见遗物回声的旧物修复师，驱动力是找回失踪的姐姐，致命缺陷是把所有责任都揽在自己身上。",
      "核心冲突是她必须在第七次退潮前破解街道规则，否则姐姐会被城市彻底遗忘；对立面是利用遗忘牟利的港务财团负责人陆沉舟。",
      "催化事件是一台送修的盐蚀录音机播放出姐姐当天留下的求救；主题探讨记忆是否决定一个人的存在，以及放手是否等于背叛。",
      "全书规划 12 章，请根据这些设定继续创作。",
    ].join(" ");
    const input = await $('textarea[aria-label="输入灵感对话内容"]');
    await input.waitForDisplayed();
    await input.setValue(idea);
    await clickButton(/^发送/);
    await $("//button[normalize-space()='等待回复']").waitForDisplayed();
    await browser.waitUntil(
      async () => !(await $("//button[normalize-space()='等待回复']").isExisting()),
      { timeout: 480_000, interval: 2_000, timeoutMsg: "灵感对话回复未完成" },
    );

    const generateBlueprint = await $("button*=生成首版蓝图");
    await generateBlueprint.waitForClickable();
    await generateBlueprint.click();
    await waitAndCloseProgress("生成首版蓝图");
    const confirmationText = await $("body").getText();
    assert.match(confirmationText, /故事蓝图/);
    assert.match(confirmationText, /潮痕|沈汐/);
    assert.doesNotMatch(confirmationText, /暂定蓝图|候选蓝图|蓝图候选/);
    await clickButton("确认进入项目");
    await browser.waitUntil(
      () => browser.getUrl().then((url) => url.includes(`/novel/${projectId}`)),
      { timeout: 30_000, timeoutMsg: "正式蓝图生成后未进入项目详情" },
    );

    await openTab("大纲");
    await openTab("分部大纲");
    const partSection = await $("//section[.//h2[normalize-space()='分部大纲']]");
    const partInputs = await partSection.$$("input[type=number]");
    await partInputs[0].setValue("1");
    await partInputs[1].setValue("12");
    await clickButton("生成分部", partSection);
    await waitAndCloseProgress("生成分部大纲");
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 部")),
      { timeout: 30_000, timeoutMsg: "AI 辅助创作未写入正式分部大纲" },
    );

    await openTab("章节大纲");
    const chapterSection = await $("//section[.//h2[normalize-space()='章节大纲']]");
    await (await chapterSection.$("input[type=number]")).setValue("1");
    await clickButton("生成章纲", chapterSection);
    await waitAndCloseProgress("生成章节大纲");
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 章")),
      { timeout: 30_000, timeoutMsg: "AI 辅助创作未写入正式章节大纲" },
    );

    await generateChapterBody(projectId, "AI 辅助创作未生成正式章节正文");
  }).timeout(1_800_000);

  it("自由创作可以跳过灵感，直接建立全部正式创作资料", async () => {
    assert.ok(freeProjectId, "自由创作项目尚未建立");
    await browser.url(`/novel/${freeProjectId}`);
    await clickButton("直接录入蓝图");
    await (await dialogField("题材")).setValue("都市悬疑");
    await (await dialogField("总章节数")).setValue("12");
    await (await dialogField("一句话概要", "textarea")).setValue(
      "失忆记者追查一封来自未来的信。",
    );
    await (await dialogField("完整梗概", "textarea")).setValue(
      "记者林雾在旧车站发现一封预告案件的信，并从钟表与雨夜站台中追查自己的失忆真相。",
    );
    await clickButton("保存为正式蓝图", await currentDialog());
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("失忆记者追查一封来自未来的信")),
      { timeoutMsg: "正式蓝图未显示在概览页" },
    );

    await openTab("角色档案");
    await clickButton("新建角色");
    await (await dialogField("姓名")).setValue("林雾");
    await (await dialogField("性格", "textarea")).setValue("冷静、执拗");
    await (await dialogField("背景", "textarea")).setValue("调查记者，缺失三年记忆");
    await clickButton("保存", await currentDialog());
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("林雾")),
      { timeoutMsg: "正式角色档案未显示" },
    );

    await openTab("物体档案");
    await clickButton("新建档案");
    await (await dialogField("名称")).setValue("停摆怀表");
    await clickButton("保存", await currentDialog());
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("停摆怀表")),
      { timeout: 5_000, timeoutMsg: "正式物体档案未创建" },
    );
    await closeDialog();

    await openTab("场景档案");
    await clickButton("新建档案");
    await (await dialogField("名称")).setValue("废弃北站");
    await clickButton("保存", await currentDialog());
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("废弃北站")),
      { timeoutMsg: "正式场景档案未创建" },
    );
    await closeDialog();

    await openTab("大纲");
    await openTab("分部大纲");
    const partCreate = await $(
      "//section[.//h2[normalize-space()='分部大纲']]//button[contains(., '直接录入')]",
    );
    await partCreate.click();
    await (await dialogField("结束章节")).setValue("6");
    await (await dialogField("分部标题")).setValue("来信");
    await (await dialogField("分部摘要", "textarea")).setValue("发现未来来信并锁定北站线索。");
    await clickButton("保存正式分部", await currentDialog());
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 部 · 来信")),
      { timeoutMsg: "正式分部大纲未显示" },
    );

    await openTab("章节大纲");
    const chapterCreate = await $(
      "//section[.//h2[normalize-space()='章节大纲']]//button[contains(., '直接录入')]",
    );
    await chapterCreate.click();
    await (await dialogField("章节标题")).setValue("雨夜来信");
    await (await dialogField("章节摘要", "textarea")).setValue("林雾在北站收到第一封未来来信。");
    await clickButton("保存正式章纲", await currentDialog());
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 章 · 雨夜来信")),
      { timeoutMsg: "正式章节大纲未显示" },
    );
  });

  it("自由创作可以从无正文的稀疏正式资料归纳蓝图并生成正文", async () => {
    await browser.url("/");
    const projectId = await createProject("自由创作", "/novel/");

    await openTab("角色档案");
    await clickButton("新建角色");
    await (await dialogField("姓名")).setValue("顾潮");
    await clickButton("保存", await currentDialog());

    await openTab("物体档案");
    await clickButton("新建档案");
    await (await dialogField("名称")).setValue("倒走怀表");
    await clickButton("保存", await currentDialog());
    await closeDialog();

    await openTab("场景档案");
    await clickButton("新建档案");
    await (await dialogField("名称")).setValue("潮汐钟楼");
    await clickButton("保存", await currentDialog());
    await closeDialog();

    await openTab("大纲");
    await openTab("章节大纲");
    const chapterCreate = await $(
      "//section[.//h2[normalize-space()='章节大纲']]//button[contains(., '直接录入')]",
    );
    await chapterCreate.click();
    await (await dialogField("章节标题")).setValue("午夜逆潮");
    await (await dialogField("章节摘要", "textarea")).setValue(
      "顾潮在潮汐钟楼发现倒走怀表，决定追查失踪的守钟人。",
    );
    await clickButton("保存正式章纲", await currentDialog());

    const details = await openMaterialSummary();
    await clickButton("归纳正式蓝图", details);
    await waitAndCloseProgress("蓝图已归纳并写入正式资料");
    await openTab("项目概览");

    const pageText = await $("body").getText();
    assert.doesNotMatch(pageText, /暂定蓝图|候选蓝图|蓝图候选/);
    assert.equal(await $("button*=确认更新蓝图").isExisting(), false);

    await generateChapterBody(projectId, "自由创作项目未生成正式章节正文");
  }).timeout(600_000);

  it("已有第 7 章正文可以跳过前序空白章节并同步局部资料", async () => {
    assert.ok(freeProjectId, "自由创作项目尚未建立");
    await browser.url(`/novel/${freeProjectId}`);
    await openTab("已生成章节");
    await clickButton("导入章节");
    await (await dialogField("章节序号")).setValue("7");
    await (await dialogField("标题")).setValue("钟下车票");
    await (await dialogField("正文", "textarea")).setValue(
      [
        "雨夜，调查记者林雾再次来到废弃北站。站台尽头的修表匠周砚叫住她，准确说出了她失去的三年记忆。",
        "周砚递来一张从未发行过的黑色车票。车票背面写着：钟楼地下室，二十三点十七分。",
        "林雾掀开站台旧钟下的暗门，和周砚进入钟楼地下室。她取出停摆怀表，表针竟向前跳动了一秒。",
        "周砚警告她：当怀表再次走动，未来来信的寄件人就会从现实中消失。林雾把黑色车票收进口袋，决定隐瞒这次会面。",
      ].join("\n\n"),
    );
    await clickButton("确认导入", await currentDialog());
    const chapterRow = await $("//button[contains(., '第 7 章')]");
    await chapterRow.waitForClickable({ timeout: 60_000 });
    await chapterRow.click();
    await browser.waitUntil(
      () => browser.getUrl().then((url) => url.includes(`/write/${freeProjectId}?chapter=7`)),
      { timeout: 60_000, timeoutMsg: "第 7 章未进入写作桌" },
    );
    await waitForMaterialSync({ timeoutMessage: "第 7 章正文资料同步未完成" });
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("生成分析")),
      { timeout: 30_000, timeoutMsg: "第 7 章分析入口不可用" },
    );
    const beforeAnalysis = await $("body").getText();
    assert.equal(beforeAnalysis.includes("请先分析第 1 章"), false);
    assert.equal(beforeAnalysis.includes("必须先完成前序章节"), false);

    await clickButton("生成分析");
    await waitAndCloseProgress("任务已完成");
    await waitForMaterialSync({
      all: true,
      timeoutMessage: "第 7 章分析工作流未完成全部资料同步",
    });
    await browser.waitUntil(
      () =>
        $("body")
          .getText()
          .then(
            (text) =>
              text.includes("当前正文的资料更新") &&
              text.includes("本次分析：") &&
              text.includes("第 7 章"),
          ),
      {
        timeout: 480_000,
        interval: 2_000,
        timeoutMsg: "第 7 章真实分析未生成资料更新结果",
      },
    );
    const panel = await $("section[aria-label='本章资料同步状态']");
    const details = await panel.$("details");
    if (await details.isExisting() && !(await details.getAttribute("open"))) {
      await details.$("summary").click();
    }
    const analysisText = await $("body").getText();
    assert.equal(analysisText.includes("生成本章章纲"), true);
    assert.equal(analysisText.includes("周砚"), true);
    assert.equal(analysisText.includes("黑色车票"), true);
    assert.equal(analysisText.includes("钟楼地下室"), true);
    assert.equal(analysisText.includes("蓝图按项目周期归纳"), true);
    const detailText = await panel.getText();
    assert.match(detailText, /同步角色状态「林雾」/);
    assert.match(detailText, /同步角色状态「周砚」/);
    assert.doesNotMatch(detailText, /角色档案同步范围不完整/);

    await browser.url(`/novel/${freeProjectId}`);
    await openTab("角色档案");
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("周砚")),
      { timeout: 30_000, timeoutMsg: "章节分析生成的角色档案未显示" },
    );
    await openTab("物体档案");
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("黑色车票")),
      { timeout: 30_000, timeoutMsg: "章节分析生成的物体档案未显示" },
    );
    await openTab("场景档案");
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("钟楼地下室")),
      { timeout: 30_000, timeoutMsg: "章节分析生成的场景档案未显示" },
    );
    await openTab("项目概览");
    await browser.waitUntil(
      () =>
        $("body")
          .getText()
          .then((text) => text.includes("失忆记者追查一封来自未来的信")),
      { timeout: 30_000, timeoutMsg: "章节分析后正式蓝图未保持原值" },
    );
    await openTab("大纲");
    await openTab("分部大纲");
    await waitForBodyText("第 1 部 · 来信");
    await openTab("章节大纲");
    await waitForBodyText("第 7 章");
  }).timeout(600_000);

  it("用户主动归纳分部后确认写入正式资料，并保留已有稀疏章纲", async () => {
    assert.ok(freeProjectId, "自由创作项目尚未建立");
    await browser.url(`/novel/${freeProjectId}`);
    const details = await openMaterialSummary();
    await details.$('button[role="combobox"]').click();
    const option = await $('//div[@role="option" and normalize-space(.)="分部大纲"]');
    await option.waitForClickable();
    await option.click();
    await clickButton("生成分部大纲候选", details);
    await waitAndCloseProgress("分部大纲候选（尚未写入正式资料）");
    await waitForBodyText("分部大纲候选（尚未写入正式资料）");
    await clickButton("确认更新分部大纲", details);
    const confirm = await currentDialog();
    assert.match(await confirm.getText(), /候选未覆盖的分部会保留/);
    await clickButton(/^更新分部大纲\s/, confirm);
    await waitForNoDialog(60_000);
    await waitForBodyText("分部大纲候选已应用");

    await browser.refresh();
    await openTab("大纲");
    await openTab("章节大纲");
    await waitForBodyText("第 1 章");
    await waitForBodyText("第 7 章");
    assert.match(await $("body").getText(), /林雾在北站收到第一封未来来信/);
  }).timeout(600_000);

  it("用户主动更新蓝图后直接形成正式版本，不出现候选蓝图", async () => {
    await browser.url(`/novel/${freeProjectId}`);
    const details = await openMaterialSummary();
    await clickButton("归纳正式蓝图", details);
    await waitAndCloseProgress("蓝图已归纳并写入正式资料");
    await waitForBodyText("本次正式蓝图更新");
    assert.equal(await $("button*=确认更新蓝图").isExisting(), false);
    await browser.refresh();
    await openTab("项目概览");
    await waitForBodyText("林雾");
    assert.doesNotMatch(await $("body").getText(), /暂定蓝图|候选蓝图|蓝图候选/);
  }).timeout(600_000);
});
