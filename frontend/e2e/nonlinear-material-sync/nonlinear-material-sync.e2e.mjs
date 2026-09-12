import assert from "node:assert/strict";

let freeProjectId = null;
const llmApiKey = process.env.A4N_NONLINEAR_LLM_API_KEY;
const llmProvider = process.env.A4N_NONLINEAR_LLM_PROVIDER;
const llmBaseUrl = process.env.A4N_NONLINEAR_LLM_BASE_URL;
const llmModel = process.env.A4N_NONLINEAR_LLM_MODEL;

const buttonByText = async (text) => {
  const button = await $("button*=" + text);
  await button.waitForClickable();
  return button;
};

const createProject = async (mode, expectedRoute) => {
  await (await buttonByText("创建项目")).click();
  await (await buttonByText(mode)).click();
  await browser.waitUntil(
    () => browser.getUrl().then((url) => url.includes(expectedRoute)),
    { timeoutMsg: `${mode} 未进入 ${expectedRoute}` },
  );
  const url = await browser.getUrl();
  const projectId = Number(url.match(/\/(?:inspiration|novel)\/(\d+)/)?.[1]);
  assert.ok(Number.isSafeInteger(projectId) && projectId > 0, `${mode} 缺少项目 ID`);
  return projectId;
};

const currentDialog = async () => {
  const dialog = await $("[role=dialog]");
  await dialog.waitForDisplayed();
  return dialog;
};

const dialogField = async (label, tagName = "input") => {
  await currentDialog();
  const field = await $(
    `//*[@role='dialog']//label[contains(normalize-space(.), "${label}")]//${tagName}`
      + ` | //*[@role='dialog']//label[contains(normalize-space(.), "${label}")]/following-sibling::${tagName}[1]`,
  );
  await field.waitForDisplayed();
  return field;
};

const openTab = async (accessibleLabel) => {
  await browser.waitUntil(() =>
    browser.execute(
      (label) => Boolean(document.querySelector(`[role=tab][aria-label="${label}"]`)),
      accessibleLabel,
    ),
  );
  const opened = await browser.execute((label) => {
    const tab = document.querySelector(`[role=tab][aria-label="${label}"]`);
    if (!(tab instanceof HTMLElement)) return false;
    tab.scrollIntoView({ block: "center", inline: "center" });
    tab.click();
    return true;
  }, accessibleLabel);
  assert.equal(opened, true, `无法打开栏目：${accessibleLabel}`);
  await browser.waitUntil(() =>
    browser.execute(
      (label) =>
        document.querySelector(`[role=tab][aria-label="${label}"]`)?.getAttribute(
          "aria-selected",
        ) === "true",
      accessibleLabel,
    ),
  );
};

const closeDialog = async () => {
  const closeButton = await $("[role=dialog] button[aria-label='关闭弹窗']");
  await closeButton.waitForClickable();
  await closeButton.click();
};

const closeDialogIfOpen = async () => {
  const closed = await browser.execute(() => {
    const button = document.querySelector(
      "[role=dialog] button[aria-label='关闭弹窗']",
    );
    if (!(button instanceof HTMLElement)) return false;
    button.click();
    return true;
  });
  if (closed) {
    await browser.waitUntil(() =>
      browser.execute(() => !document.querySelector("[role=dialog]")),
    );
  }
};

const closeSuccessfulProgress = async (
  successText,
  timeout = 480_000,
  closeText = "关闭",
) => {
  await (await $("[role=dialog]")).waitForDisplayed({ timeout: 30_000 });
  let progressText = "";
  await browser.waitUntil(
    async () => {
      progressText = await (await $("[role=dialog]")).getText();
      return progressText.includes(successText)
        || /生成失败|任务失败|调用失败|分析失败|同步失败|已取消|已停止/.test(
          progressText,
        );
    },
    { timeout, interval: 2_000, timeoutMsg: `任务未完成：${successText}` },
  );
  assert.equal(progressText.includes(successText), true, progressText);
  const dialog = await $("[role=dialog]");
  const close = await dialog.$(`button*=${closeText}`);
  await close.waitForClickable();
  await close.click();
  await dialog.waitForDisplayed({ reverse: true, timeout: 30_000 });
};

const runGenerationWithRetry = async ({
  start,
  successText,
  closeText = "关闭",
  attempts = 3,
  retryDelayMs = 30_000,
}) => {
  let lastError = null;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    await start();
    try {
      await closeSuccessfulProgress(successText, 480_000, closeText);
      return;
    } catch (error) {
      lastError = error;
      const dialog = await $("[role=dialog]");
      const text = (await dialog.isExisting()) ? await dialog.getText() : "";
      const retryable = /模型流读取失败|LLM API 流式请求失败|unexpected EOF|error sending request|429 Too Many Requests|服务限流|请求数限制/i.test(text);
      if (!retryable || attempt === attempts) throw error;
      const close = await dialog.$("button*=关闭");
      await close.waitForClickable();
      await close.click();
      await dialog.waitForDisplayed({ reverse: true, timeout: 30_000 });
      if (retryDelayMs > 0) await browser.pause(retryDelayMs);
    }
  }
  throw lastError;
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
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    await browser.waitUntil(
      async () => {
        const panel = await $("section[aria-label='本章资料同步状态']");
        if (!(await panel.isExisting())) return false;
        statuses = await browser.execute(() =>
          [...document.querySelectorAll("section[aria-label='本章资料同步状态'] dd")]
            .map((row) => row.textContent?.trim() ?? ""),
        );
        return statuses.includes("同步失败")
          || (all
            ? statuses.length === 5 && statuses.every((status) => status === "已完成")
            : statuses[0] === "已完成");
      },
      { timeout: 180_000, interval: 2_000, timeoutMsg: timeoutMessage },
    );
    if (!statuses.includes("同步失败")) break;
    if (!all || attempt === 3) break;
    await browser.waitUntil(
      async () => {
        const pageText = await $("body").getText();
        const retryable = /429 Too Many Requests|服务限流|请求数限制|模型流读取失败|LLM API 流式请求失败|unexpected EOF|error sending request/i.test(pageText);
        const continueSync = await $("button*=继续同步");
        return retryable && await continueSync.isClickable();
      },
      {
        timeout: 30_000,
        interval: 1_000,
        timeoutMsg: "可恢复的章节分析失败未提供继续同步入口",
      },
    );
    await browser.pause(30_000);
    await runGenerationWithRetry({
      start: async () => (await $("button*=继续同步")).click(),
      successText: "章节分析已继续完成",
      retryDelayMs: 30_000,
    });
    await openWritingAnalysis();
  }
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
  for (let attempt = 0; attempt < 2; attempt += 1) {
    const start = await $("//aside[@aria-label='写作助手']//button[contains(., '开始生成')]");
    await start.waitForClickable({ timeout: 30_000 });
    await start.click();
    const confirm = await $("[role=dialog]");
    await confirm.waitForDisplayed();
    await (await confirm.$(
      "//button[normalize-space()='生成' or normalize-space()='重新生成']",
    )).click();
    await confirm.waitForDisplayed({ reverse: true, timeout: 30_000 });
    await (await $(
      "//aside[@aria-label='写作助手']//button[contains(., '取消生成')]",
    )).waitForDisplayed({ timeout: 30_000 });
    await browser.waitUntil(
      () => browser.execute(() => {
        const buttons = [...document.querySelectorAll("button")];
        return buttons.some((button) => button.textContent?.includes("开始生成"))
          && !buttons.some((button) => button.textContent?.includes("取消生成"));
      }),
      { timeout: 480_000, interval: 2_000, timeoutMsg: timeoutMessage },
    );
    const contentLength = await browser.execute(() => {
      const editor = document.querySelector("textarea[aria-label='章节正文']");
      return editor instanceof HTMLTextAreaElement ? editor.value.trim().length : 0;
    });
    if (contentLength >= 500) {
      await waitForMaterialSync({ timeoutMessage: `${timeoutMessage}：正文索引未完成` });
      return;
    }
  }
  assert.fail(timeoutMessage);
};

describe("非线性创作资料同步", () => {
  before(async () => {
    if (!llmApiKey) return;
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
    await (await buttonByText("创建项目")).click();
    const modeDialog = await currentDialog();
    const modeLabels = await browser.execute(() =>
      [...document.querySelectorAll("[role=dialog] button")]
        .map((button) => button.textContent?.trim() ?? "")
        .filter((text) =>
          ["AI 辅助创作", "自由创作", "视频画布项目"].some((label) =>
            text.includes(label),
          ),
        ),
    );
    const dialogText = await modeDialog.getText();
    assert.equal(dialogText.includes("AI 辅助创作"), true);
    assert.equal(dialogText.includes("自由创作"), true);
    assert.equal(dialogText.includes("视频画布项目"), true);
    assert.equal(dialogText.includes("小说项目"), false);
    assert.equal(dialogText.includes("剧本项目"), false);
    assert.equal(modeLabels.length, 3);

    await browser.keys("Escape");
    const aiProjectId = await createProject("AI 辅助创作", "/inspiration/");

    await browser.url("/");
    freeProjectId = await createProject("自由创作", "/novel/");
    assert.notEqual(aiProjectId, freeProjectId);
  });

  it("AI 辅助创作可以从 idea 完成正式蓝图、大纲和正文全链路", async () => {
    assert.ok(llmApiKey, "真实模型验收缺少临时 LLM API Key");
    await browser.url("/");
    const projectId = await createProject("AI 辅助创作", "/inspiration/");
    const prompts = [
      [
        "我已经想清楚全部核心要素，请直接整理并结束概念对话。",
        "作品暂名《潮痕回声》，核心火花是海滨城每逢退潮就会出现一段只属于失踪者的街道。",
        "类型是都市奇幻悬疑，基调克制、潮湿而温暖；文风采用第三人称限知、简洁有画面感的现实主义笔触。",
        "主角沈汐是能听见遗物回声的旧物修复师，驱动力是找回失踪的姐姐，致命缺陷是把所有责任都揽在自己身上。",
        "核心冲突是她必须在第七次退潮前破解街道规则，否则姐姐会被城市彻底遗忘；对立面是利用遗忘牟利的港务财团负责人陆沉舟。",
        "催化事件是一台送修的盐蚀录音机播放出姐姐当天留下的求救；主题探讨记忆是否决定一个人的存在，以及放手是否等于背叛。",
        "全书规划 12 章。信息已经完整，不需要继续追问；请给出标题建议并将 conversation_state.genre 设为都市奇幻悬疑、chapter_count 设为 12，标记对话完成。",
      ].join(" "),
      "上述核心火花、类型基调、文风、主角、冲突、对立面、催化事件、主题和 12 章篇幅均已确认，请结束概念对话并标记完成。",
    ];

    await (await $("textarea")).waitForDisplayed({ timeout: 30_000 });
    for (const prompt of prompts) {
      await (await $("textarea")).setValue(prompt);
      await (await $("//button[normalize-space()='发送']")).click();
      await (await $("//button[normalize-space()='等待回复']")).waitForDisplayed({ timeout: 30_000 });
      await browser.waitUntil(
        () => $("//button[normalize-space()='等待回复']")
          .then((button) => button.isExisting())
          .then((existing) => !existing),
        { timeout: 480_000, interval: 2_000, timeoutMsg: "灵感对话回复未完成" },
      );
      if (await $("button*=生成蓝图").then((button) => button.isClickable())) break;
    }

    const generateBlueprint = await $("button*=生成蓝图");
    assert.equal(await generateBlueprint.isClickable(), true, "完整 idea 未结束灵感对话");
    await runGenerationWithRetry({
      start: async () => (await $("button*=生成蓝图")).click(),
      successText: "首版蓝图已生成",
      closeText: "查看蓝图",
    });
    const confirmationText = await $("body").getText();
    assert.match(confirmationText, /故事蓝图/);
    assert.match(confirmationText, /潮痕|沈汐/);
    assert.doesNotMatch(confirmationText, /暂定蓝图|候选蓝图|蓝图候选/);
    await (await buttonByText("确认进入项目")).click();
    await browser.waitUntil(
      () => browser.getUrl().then((url) => url.includes(`/novel/${projectId}`)),
      { timeout: 30_000, timeoutMsg: "正式蓝图生成后未进入项目详情" },
    );

    await openTab("章节大纲");
    const partSection = await $("//section[.//h2[normalize-space()='分部大纲']]");
    const partInputs = await partSection.$$("input[type=number]");
    await partInputs[0].setValue("1");
    await partInputs[1].setValue("12");
    await runGenerationWithRetry({
      start: async () => (await $("//section[.//h2[normalize-space()='分部大纲']]//button[contains(., '生成分部')]")).click(),
      successText: "1 个分部",
    });
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 部")),
      { timeout: 30_000, timeoutMsg: "AI 辅助创作未写入正式分部大纲" },
    );

    const chapterSection = await $("//section[.//h2[normalize-space()='章节大纲']]");
    await (await chapterSection.$("input[type=number]")).setValue("1");
    await runGenerationWithRetry({
      start: async () => (await $("//section[.//h2[normalize-space()='章节大纲']]//button[contains(., '生成章纲')]")).click(),
      successText: "1 个章纲",
    });
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 章")),
      { timeout: 30_000, timeoutMsg: "AI 辅助创作未写入正式章节大纲" },
    );

    await generateChapterBody(projectId, "AI 辅助创作未生成正式章节正文");
  }).timeout(1_800_000);

  it("自由创作可以跳过灵感，直接建立全部正式创作资料", async () => {
    assert.ok(freeProjectId, "自由创作项目尚未建立");
    await browser.url(`/novel/${freeProjectId}`);
    await (await buttonByText("直接录入蓝图")).click();
    await (await dialogField("题材")).setValue("都市悬疑");
    await (await dialogField("总章节数")).setValue("12");
    await (await dialogField("一句话概要", "textarea")).setValue(
      "失忆记者追查一封来自未来的信。",
    );
    await (await dialogField("完整梗概", "textarea")).setValue(
      "记者林雾在旧车站发现一封预告案件的信，并从钟表与雨夜站台中追查自己的失忆真相。",
    );
    await (await buttonByText("保存为正式蓝图")).click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("失忆记者追查一封来自未来的信")),
      { timeoutMsg: "正式蓝图未显示在概览页" },
    );

    await openTab("角色档案");
    await (await buttonByText("新建角色")).click();
    await (await dialogField("姓名")).setValue("林雾");
    await (await dialogField("性格", "textarea")).setValue("冷静、执拗");
    await (await dialogField("背景", "textarea")).setValue("调查记者，缺失三年记忆");
    await (await buttonByText("保存")).click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("林雾")),
      { timeoutMsg: "正式角色档案未显示" },
    );

    await openTab("物体档案");
    await (await buttonByText("新建档案")).click();
    await (await dialogField("名称")).setValue("停摆怀表");
    await (await buttonByText("保存")).click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("停摆怀表")),
      { timeout: 5_000, timeoutMsg: "正式物体档案未创建" },
    );
    await closeDialogIfOpen();

    await openTab("场景档案");
    await (await buttonByText("新建档案")).click();
    await (await dialogField("名称")).setValue("废弃北站");
    await (await buttonByText("保存")).click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("废弃北站")),
      { timeoutMsg: "正式场景档案未创建" },
    );
    await closeDialogIfOpen();

    await openTab("章节大纲");
    const partCreate = await $(
      "//section[.//h2[normalize-space()='分部大纲']]//button[contains(., '直接录入')]",
    );
    await partCreate.click();
    await (await dialogField("结束章节")).setValue("6");
    await (await dialogField("分部标题")).setValue("来信");
    await (await dialogField("分部摘要", "textarea")).setValue("发现未来来信并锁定北站线索。");
    await (await buttonByText("保存正式分部")).click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 部 · 来信")),
      { timeoutMsg: "正式分部大纲未显示" },
    );

    const chapterCreate = await $(
      "//section[.//h2[normalize-space()='章节大纲']]//button[contains(., '直接录入')]",
    );
    await chapterCreate.click();
    await (await dialogField("章节标题")).setValue("雨夜来信");
    await (await dialogField("章节摘要", "textarea")).setValue("林雾在北站收到第一封未来来信。");
    await (await buttonByText("保存正式章纲")).click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("第 1 章 · 雨夜来信")),
      { timeoutMsg: "正式章节大纲未显示" },
    );
  });

  it("自由创作可以从无正文的稀疏正式资料归纳蓝图并生成正文", async () => {
    assert.ok(llmApiKey, "真实模型验收缺少临时 LLM API Key");
    await browser.url("/");
    const projectId = await createProject("自由创作", "/novel/");

    await openTab("角色档案");
    await (await buttonByText("新建角色")).click();
    await (await dialogField("姓名")).setValue("顾潮");
    await (await buttonByText("保存")).click();

    await openTab("物体档案");
    await (await buttonByText("新建档案")).click();
    await (await dialogField("名称")).setValue("倒走怀表");
    await (await buttonByText("保存")).click();
    await closeDialogIfOpen();

    await openTab("场景档案");
    await (await buttonByText("新建档案")).click();
    await (await dialogField("名称")).setValue("潮汐钟楼");
    await (await buttonByText("保存")).click();
    await closeDialogIfOpen();

    await openTab("章节大纲");
    const chapterCreate = await $(
      "//section[.//h2[normalize-space()='章节大纲']]//button[contains(., '直接录入')]",
    );
    await chapterCreate.click();
    await (await dialogField("章节标题")).setValue("午夜逆潮");
    await (await dialogField("章节摘要", "textarea")).setValue(
      "顾潮在潮汐钟楼发现倒走怀表，决定追查失踪的守钟人。",
    );
    await (await buttonByText("保存正式章纲")).click();

    await openTab("项目概览");
    await runGenerationWithRetry({
      start: async () => (await buttonByText("依据现有资料归纳蓝图")).click(),
      successText: "蓝图已归纳并写入正式资料",
    });

    const pageText = await $("body").getText();
    assert.doesNotMatch(pageText, /暂定蓝图|候选蓝图|蓝图候选/);
    assert.equal(await $("button*=确认更新蓝图").isExisting(), false);

    await generateChapterBody(projectId, "自由创作项目未生成正式章节正文");
  }).timeout(600_000);

  it("已有第 7 章正文可以跳过前序空白章节并同步局部资料", async () => {
    assert.ok(freeProjectId, "自由创作项目尚未建立");
    await browser.url(`/novel/${freeProjectId}`);
    await openTab("已生成章节");
    await (await buttonByText("导入章节")).click();
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
    await (await buttonByText("确认导入")).click();
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

    await (await buttonByText("生成分析")).click();
    await closeSuccessfulProgress("结构化分析已保存");
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
    const analysisText = await $("body").getText();
    assert.equal(analysisText.includes("生成本章章纲"), true);
    assert.equal(analysisText.includes("周砚"), true);
    assert.equal(analysisText.includes("黑色车票"), true);
    assert.equal(analysisText.includes("钟楼地下室"), true);
    assert.equal(analysisText.includes("分部大纲和蓝图只在你主动操作时更新"), true);

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
    await openTab("章节大纲");
    const outlineText = await $("body").getText();
    assert.equal(outlineText.includes("第 1 部 · 来信"), true);
    assert.equal(outlineText.includes("第 7 章"), true);
  }).timeout(600_000);
});
