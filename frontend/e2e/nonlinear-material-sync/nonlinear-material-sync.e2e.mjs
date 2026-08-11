import assert from "node:assert/strict";

let freeProjectId = null;

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

describe("非线性创作资料同步", () => {
  it("创建模式只保留三个入口，两个文本模式只改变首次落点", async () => {
    await browser.url("/");
    await browser.pause(1_000);
    const initial = await browser.execute(() => ({
      url: location.href,
      text: document.body?.innerText ?? "",
      buttons: [...document.querySelectorAll("button")].map(
        (button) => button.textContent?.trim() ?? "",
      ),
    }));
    process.stdout.write(`A4N_NONLINEAR_INITIAL=${JSON.stringify(initial)}\n`);
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

    process.stdout.write(
      `A4N_NONLINEAR_PROJECTS=${JSON.stringify({ aiProjectId, freeProjectId })}\n`,
    );
  });

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
    await browser.pause(1_000);
    const objectSnapshot = await browser.execute(() => ({
      url: location.href,
      text: document.body?.innerText ?? "",
      dialogs: [...document.querySelectorAll("[role=dialog]")].map(
        (dialog) => dialog.textContent ?? "",
      ),
      selectedTab: document.querySelector("[role=tab][aria-selected=true]")?.textContent ?? "",
    }));
    process.stdout.write(`A4N_NONLINEAR_OBJECT=${JSON.stringify(objectSnapshot)}\n`);
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

    process.stdout.write(`A4N_NONLINEAR_FORMAL_PROJECT=${freeProjectId}\n`);
  });

  it("已有第 7 章正文可以跳过前序空白章节并同步局部资料", async () => {
    assert.ok(freeProjectId, "自由创作项目尚未建立");
    process.stdout.write("A4N_NONLINEAR_STAGE=chapter-import-start\n");
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
    process.stdout.write("A4N_NONLINEAR_STAGE=writing-desk-opened\n");
    await browser.waitUntil(
      () =>
        $("body")
          .getText()
          .then(
            (text) =>
              !text.includes("正文已保存，正在使旧角色状态、场景和检索索引失效") &&
              !text.includes("正文已保存，派生资料仍在同步"),
          ),
      { timeout: 120_000, timeoutMsg: "第 7 章正文资料同步未完成" },
    );
    process.stdout.write("A4N_NONLINEAR_STAGE=derived-sync-completed\n");

    const analysisTab = await $(
      "//nav[@aria-label='章节工作区']//button[normalize-space()='分析']",
    );
    await analysisTab.waitForClickable();
    await analysisTab.click();
    await browser.waitUntil(
      () => $("body").getText().then((text) => text.includes("生成分析")),
      { timeout: 30_000, timeoutMsg: "第 7 章分析入口不可用" },
    );
    process.stdout.write("A4N_NONLINEAR_STAGE=analysis-ready\n");
    const beforeAnalysis = await $("body").getText();
    assert.equal(beforeAnalysis.includes("请先分析第 1 章"), false);
    assert.equal(beforeAnalysis.includes("必须先完成前序章节"), false);

    await (await buttonByText("生成分析")).click();
    process.stdout.write("A4N_NONLINEAR_STAGE=analysis-started\n");
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

    process.stdout.write(`A4N_NONLINEAR_ANALYZED_PROJECT=${freeProjectId}\n`);
  }).timeout(600_000);
});
