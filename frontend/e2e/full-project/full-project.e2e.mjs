import assert from "node:assert/strict";
import { readdir } from "node:fs/promises";
import { visibleElements, firstVisibleContaining, buttonMatching, clickButton, currentDialog, fieldByLabel, setField, waitForBodyText, waitForBodyMatch, waitForRoute, waitForNoDialog, closeDialog, waitAndCloseProgress, tab, sectionContaining, createProject, prepareSession } from "./user-flow.mjs";

async function enterBlueprint(projectId) {
  await browser.url(`/novel/${projectId}`);
  await waitForBodyText("项目概况");
  await clickButton("直接录入蓝图");
  const dialog = await currentDialog();
  await setField("题材", "都市时间悬疑", "input", dialog);
  await setField("总章节数", "1", "input", dialog);
  await setField("写作风格", "克制、具象、以细节推进谜团", "input", dialog);
  await setField("情感基调", "紧张中保留温暖", "input", dialog);
  await setField("目标读者", "喜欢都市悬疑与轻科幻的成年读者", "input", dialog);
  await setField("一句话概要", "一封来自未来的信迫使林岚进入钟楼。", "textarea", dialog);
  await setField("完整梗概", "暴雨夜，林岚在旧车站发现来自未来的信，并追查每天慢七分钟的实验钟。", "textarea", dialog);
  await clickButton("保存为正式蓝图", dialog);
  await waitForBodyText("一封来自未来的信迫使林岚进入钟楼", 60_000);
}

async function createProfiles() {
  await tab("角色档案");
  await clickButton("新建角色");
  let dialog = await currentDialog();
  await setField("姓名", "林岚", "input", dialog);
  await setField("性格", "冷静、敏锐", "textarea", dialog);
  await setField("背景", "档案修复师，追查时间异常", "textarea", dialog);
  await clickButton("保存", dialog);
  await waitForBodyText("林岚", 60_000);

  await tab("物体档案");
  await clickButton("新建档案");
  dialog = await currentDialog();
  await setField("名称", "旧怀表", "input", dialog);
  await clickButton("保存", dialog);
  await waitForBodyText("旧怀表", 60_000);
  await closeDialog();

  await tab("场景档案");
  await clickButton("新建档案");
  dialog = await currentDialog();
  await setField("名称", "旧车站", "input", dialog);
  await clickButton("保存", dialog);
  await waitForBodyText("旧车站", 60_000);
  await closeDialog();
}

async function createOutlines() {
  await tab("大纲");
  await tab("分部大纲");
  const part = await sectionContaining("分部大纲");
  await clickButton("直接录入", part);
  let dialog = await currentDialog();
  await setField("分部编号", "1", "input", dialog);
  await setField("起始章节", "1", "input", dialog);
  await setField("结束章节", "1", "input", dialog);
  await setField("分部标题", "七分钟来信", "input", dialog);
  await setField("分部主题", "面对被篡改的记忆", "input", dialog);
  await setField("分部摘要", "林岚追查未来来信和旧车站的时间偏差。", "textarea", dialog);
  await clickButton("保存正式分部", dialog);
  await waitForBodyText("七分钟来信", 60_000);

  await tab("章节大纲");
  const chapter = await sectionContaining("章节大纲");
  await clickButton("直接录入", chapter);
  dialog = await currentDialog();
  await setField("章节号", "1", "input", dialog);
  await setField("章节标题", "雨夜来信", "input", dialog);
  await setField("章节摘要", "林岚在旧车站确认七分钟时间倒退。", "textarea", dialog);
  await clickButton("保存正式章纲", dialog);
  await waitForBodyText("雨夜来信", 60_000);
}

const chapterBody = [
  "雨水沿着末班车的车窗斜斜滑下，林岚把那封没有邮戳的信翻到第三遍。信封正面写着她的名字，背面却标着十年后的日期。十二点前进入旧车站钟楼，否则整座城会失去七分钟——字迹和她一模一样。",
  "列车驶入废弃站台旁的弯道时，车厢灯忽然熄灭。黑暗里先响起一声机械钟鸣，接着所有人的手机同时亮起，时间从二十三点四十二分跳回二十三点三十五分。乘客没有察觉，仍重复着熄灯前的动作；只有林岚掌心那只旧怀表，秒针逆着刻度走了整整一圈。",
  "她在旧车站下车。雨幕后的钟楼四面表盘停在不同时间，最慢的一面恰好落后七分钟。站厅深处传来金属碰撞声，修表师周衡提着工具箱从封闭通道里走出。他看见信封角落的齿轮暗记，神色第一次失去从容。",
  "“这个记号十年前就该被烧掉了。”周衡说。",
  "林岚没有把信交给他，只展开其中最后一页。纸上新浮出一行被雨水般的墨迹浸透的字：别相信替你校准时间的人。与此同时，钟楼顶端传来第二声钟鸣。站厅广告屏里的画面倒退七秒，周衡身后的影子却没有跟着移动。林岚合上怀表，沿着封闭通道向地下走去。她知道信中的第一个预告已经发生，而距离午夜只剩十八分钟。",
].join("\n\n");

async function importChapter() {
  await tab("已生成章节");
  await clickButton("导入章节");
  const dialog = await currentDialog();
  await setField("章节序号", "1", "input", dialog);
  await setField("标题", "雨夜来信", "input", dialog);
  await setField("正文", chapterBody, "textarea", dialog);
  await clickButton("确认导入", dialog);
  await waitForBodyMatch(/章节.*导入|已导入/, 120_000);
  await waitForNoDialog(120_000);
  await browser.waitUntil(async () => {
    const text = await $("body").getText();
    return text.includes("雨夜来信") && text.includes("1");
  }, { timeout: 60_000, interval: 500 });
}

async function analyzeAndSync(projectId) {
  await browser.url(`/write/${projectId}?chapter=1`);
  let editor = null;
  await browser.waitUntil(
    async () => {
      const editors = await visibleElements("textarea[aria-label='正文'], textarea[aria-label='章节正文']");
      editor = editors[0] ?? null;
      return editor != null;
    },
    { timeout: 60_000, interval: 250, timeoutMsg: "写作桌缺少章节正文编辑器" },
  );
  await editor.waitForDisplayed({ timeout: 60_000 });
  assert.ok((await editor.getValue()).includes("林岚"));
  await tab("分析");
  await clickButton(/生成分析|重新分析|继续分析/);
  await waitAndCloseProgress("任务已完成", 480_000);
  const panelSelector = "section[aria-label='本章资料同步状态']";
  await $(panelSelector).waitForDisplayed({ timeout: 60_000 });
  await browser.waitUntil(async () => {
    try {
      const panel = await $(panelSelector);
      const statuses = await panel.$$("dd");
      if (statuses.length !== 5) return false;
      for (const item of statuses) {
        if ((await item.getText()).trim() !== "已完成") return false;
      }
      return true;
    } catch {
      return false;
    }
  }, { timeout: 480_000, interval: 2_000, timeoutMsg: "章节资料同步未全部完成" });
  const buttons = await visibleElements("button");
  let hasContinue = false;
  for (const item of buttons) {
    if ((await item.getText()).includes("继续同步")) {
      hasContinue = true;
      break;
    }
  }
  assert.equal(hasContinue, false);
  assert.doesNotMatch(await $("body").getText(), /重试资料同步/);
}

async function exerciseDirectorPlan(projectId) {
  await browser.url(`/write/${projectId}?chapter=1`);
  await waitForBodyText("正文");
  await tab("导演编排");
  const plan = await $("section.director-workspace");
  await plan.waitForDisplayed({ timeout: 60_000 });
  const frame = await $(".director-viewer").getSize();
  assert.ok(Math.abs(frame.width / frame.height - 16 / 9) < 0.02);
  assert.equal((await $$(".director-segment")).length, 1);

  await browser.execute(() => {
    const input = document.querySelector('[aria-label="播放位置"]');
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, "5000");
    input.dispatchEvent(new Event("input", { bubbles: true }));
    input.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await clickButton("添加图片点");
  await browser.waitUntil(async () => (await $$(".director-point")).length === 3);
  assert.equal((await $$(".director-segment")).length, 1, "添加图片点不能自动拆镜头");
  await (await $$(".director-point"))[1].click();
  await setField("名称", "门口中段", "input", await $(".director-inspector"));
  await clickButton("从此处分镜");
  const split = await currentDialog();
  await setField("前段描述", "镜头缓缓推近门口。", "textarea", split);
  await setField("后段描述", "人物推门进入房间。", "textarea", split);
  await clickButton("拆分镜头", split);
  await waitForNoDialog();
  await browser.waitUntil(async () => (await $$(".director-segment")).length === 2);

  await clickButton("调整整体时长");
  await $('[aria-label="整体时长（秒）"]').setValue("20");
  await browser.keys("Tab");
  await browser.waitUntil(async () => (await $(".director-ruler").getText()).includes("20s"));
  await closeDialog();
  await clickButton("保存草稿");
  await browser.waitUntil(async () => (await $(".director-save-status").getText()).includes("草稿已保存"));
  await clickButton("原文", await $(".director-tool-rail"));
  assert.match(await (await currentDialog()).getText(), /章节原文/);
  await closeDialog();
  await clickButton("收起属性");

  for (const width of [1440, 1024, 390]) {
    await browser.setWindowSize(width, 1000);
    await browser.waitUntil(async () => browser.execute(() => document.documentElement.scrollWidth === document.documentElement.clientWidth));
    await browser.saveScreenshot(`${process.env.A4N_BROWSER_E2E_ARTIFACT_DIR}/director-plan-${width}.png`);
  }
  await (await $$(".director-point"))[1].click();
  assert.match(await (await currentDialog()).getText(), /图片点属性/);
  await browser.keys("Escape");
  await waitForNoDialog();
  const playhead = await $('[aria-label="播放位置"]');
  const before = await playhead.getValue();
  await clickButton("播放分镜");
  await browser.waitUntil(async () => (await playhead.getValue()) !== before);
  await clickButton("暂停");
  await browser.setWindowSize(1440, 1000);
}

async function searchRag() {
  // 资料检索属于正文编辑上下文；分析页上的入口按产品规则保持禁用。
  await tab("正文");
  const ragButton = await buttonMatching("资料检索");
  await ragButton.click();
  const panelSelector = "aside[aria-label='资料检索']";
  const panel = await $(panelSelector);
  await panel.waitForDisplayed({ timeout: 60_000 });
  const query = await panel.$("textarea");
  await query.setValue("旧怀表和七分钟时间异常");
  await clickButton(/搜索|检索|Search/i, panel);
  await browser.waitUntil(
    async () => {
      try {
        return /结果|result|旧怀表/i.test(await $(panelSelector).getText());
      } catch {
        return false;
      }
    },
    { timeout: 120_000, interval: 1_000, timeoutMsg: "资料检索未返回结果" },
  );
  assert.match(await $(panelSelector).getText(), /旧怀表|七分钟/);
}

async function optimizeWithDomainAgent(projectId) {
  await browser.url(`/write/${projectId}?chapter=1&agent=1`);
  const consoleSelector = "section[aria-label='领域 Agent 控制台']";
  let input = null;
  await browser.waitUntil(async () => {
    try {
      const console = (await visibleElements(consoleSelector)).at(-1);
      if (!console) return false;
      input = await fieldByLabel("发送给领域 Agent 的消息", "textarea", console);
      return true;
    } catch {
      return false;
    }
  }, { timeout: 60_000, interval: 250, timeoutMsg: "领域 Agent 输入框未稳定出现" });
  assert.ok(input, "领域 Agent 输入框未找到");
  await input.setValue("请优化这一章的表达与节奏，保持剧情事实和人物设定不变。");
  await browser.waitUntil(async () => {
    try {
      const console = (await visibleElements(consoleSelector)).at(-1);
      if (!console) return false;
      const button = await buttonMatching("发送给领域 Agent", console, 0);
      if ((await button.getAttribute("disabled")) != null) return false;
      await button.click();
      return true;
    } catch {
      return false;
    }
  }, { timeout: 30_000, interval: 250, timeoutMsg: "领域 Agent 发送按钮未稳定出现" });
  await browser.waitUntil(async () => {
    try {
      const console = (await visibleElements(consoleSelector)).at(-1);
      if (!console) return false;
      const text = await console.getText();
      if (await approvePendingProposal(console, text)) return false;
      // Content optimization now returns a reviewable patch proposal; it does
      // not write the chapter back until the user approves it in the editor.
      return /已完成本章表达与节奏的复核|局部补丁提案|正文尚未写回|Agent 任务失败|Agent 任务已中断/.test(text);
    } catch {
      return false;
    }
  }, { timeout: 480_000, interval: 1_000, timeoutMsg: "正文优化子 Agent 未完成" });
  const console = (await visibleElements(consoleSelector)).at(-1);
  assert.ok(console, "领域 Agent 控制台未找到");
  assert.match(await console.getText(), /剧情事实和人物设定保持不变|局部补丁提案|正文尚未写回/);
}

async function approvePendingProposal(scope, text) {
  if (!text.includes("edit_article")) return false;
  for (const button of await visibleElements("button", scope)) {
    if ((await button.getText()) === "批准" && await button.isEnabled()) {
      await button.click();
      return true;
    }
  }
  return false;
}

async function generateImage(projectId) {
  await browser.url(`/novel/${projectId}`);
  // 等待项目详情初始 Tab 从持久化状态完成一次同步，再切换到图像。
  await waitForBodyText("导入章节", 60_000);
  await tab("项目图像");
  await browser.pause(400);
  // 项目详情页的“已生成章节”Tab 也包含“生成”文本；把按钮限制在
  // 当前页面内容区域，避免误点 Tab 导致回到章节页。
  await clickButton("生成", await $(".a4n-page-container"));
  const dialog = await currentDialog();
  await setField("提示词", "暴雨中的旧车站钟楼，冷灰色电影感，画面中有一只停在七分钟偏差的旧怀表。", "textarea", dialog);
  await clickButton("开始生成", dialog);
  await waitAndCloseProgress("生成项目图像");
  await waitForBodyMatch(/图片|图像/);
  assert.ok((await $$('img')).length > 0, "图库没有生成图片");
}

async function createDerivedProjects() {
  await browser.url("/");
  await waitForBodyText("创建项目");
  let projectRow = await firstVisibleContaining('.a4n-project-row', "未命名小说项目");
  assert.ok(projectRow, "首页找不到文本项目行");
  const more = await projectRow.$("button[aria-label='更多操作']");
  await more.click();
  const kinds = [
    ["创建漫画项目", /\/comic-project\/(\d+)/],
    ["创建视频项目", /\/video-project\/(\d+)/],
    ["创建 GALGame 项目", /\/galgame-project\/(\d+)/],
  ];
  const ids = {};
  for (const [label, route] of kinds) {
    await clickButton(label);
    let dialog = await currentDialog();
    await clickButton("下一步", dialog);
    dialog = await currentDialog();
    await setField("章节范围", "1-1", "input", dialog);
    await clickButton("创建", dialog);
    await waitForRoute(route, 120_000);
    const match = (await browser.getUrl()).match(route);
    assert.ok(match, `${label} 缺少项目 ID`);
    ids[label] = Number(match[1]);
    await browser.url("/");
    await waitForBodyText("创建项目");
    projectRow = await firstVisibleContaining('.a4n-project-row', "未命名小说项目");
    assert.ok(projectRow, `${label} 创建后找不到根项目`);
    await (await projectRow.$("button[aria-label='更多操作']")).click();
  }
  assert.equal(Object.keys(ids).length, 3);
  assert.ok(ids["创建视频项目"] > 0);
  return ids;
}

async function exerciseCanvas(videoId) {
  await browser.url(`/video-project/${videoId}`);
  await waitForBodyText("画布模式");
  await clickButton("画布模式");
  await waitForRoute(new RegExp(`/video-project/${videoId}/studio`));
  await waitForBodyText("节点");
  await browser.setWindowSize(390, 900);
  const openNodeLibrary = async () => {
    const button = await $("button[aria-label='打开节点库']");
    await button.waitForClickable({ timeout: 60_000 });
    await button.click();
  };
  for (const label of ["文本输入", "图片输入", "视频输入"]) {
    // Adding a node rerenders the toolbar; always resolve a fresh button.
    await openNodeLibrary();
    const dialog = await currentDialog();
    await clickButton(label, dialog);
    await closeDialog();
    if (label !== "文本输入") {
      // Empty media-input templates are intentionally invalid until a real
      // asset is bound. Remove them through the visible node context menu,
      // leaving a valid text node to persist and re-open.
      await browser.waitUntil(async () => {
        try {
          const node = await firstVisibleContaining('[role="group"][aria-roledescription="node"]', label);
          if (!node) return false;
          await node.click({ button: "right" });
          for (const menu of await visibleElements('[role="menu"]')) {
            if ((await menu.getText()).includes("删除节点")) return true;
          }
          return false;
        } catch {
          return false;
        }
      }, { timeout: 60_000, interval: 250, timeoutMsg: `未打开画布节点菜单：${label}` });
      await clickButton("删除节点");
      await browser.waitUntil(
        async () => !(await firstVisibleContaining('[role="group"][aria-roledescription="node"]', label)),
        { timeout: 30_000, interval: 250, timeoutMsg: `未删除无引用的画布节点：${label}` },
      );
    }
  }
  await clickButton("保存画布");
  await waitForBodyText("画布已保存", 60_000);
  await browser.refresh();
  await waitForBodyMatch(/文本输入|图片输入|视频输入/, 60_000);
  await clickButton(/领域 Agent|打开画布 Agent/);
  const agentDialog = await currentDialog();
  assert.match(await agentDialog.getText(), /画布 Agent/);
  let canvasAgent;
  await browser.waitUntil(async () => {
    try {
      const dialogs = await visibleElements('[role="dialog"]');
      const dialog = dialogs.at(-1);
      const scopes = dialog ? await visibleElements('section[aria-label="领域 Agent 控制台"]', dialog) : [];
      canvasAgent = scopes[0] ?? null;
      if (!canvasAgent) return false;
      const inputs = await visibleElements("textarea", canvasAgent);
      const input = inputs[0];
      if (!input) return false;
      return await input.isDisplayed();
    } catch {
      canvasAgent = null;
      return false;
    }
  }, { timeout: 60_000, interval: 250, timeoutMsg: "画布 Agent 输入框未稳定出现" });
  await closeDialog();
  const overflow = await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.equal(overflow, 0, `窄屏画布横向溢出 ${overflow}px`);
  await browser.setWindowSize(1440, 1000);
}

async function exerciseMainAgent(projectId) {
  await browser.url("/agents");
  await waitForBodyText("主 Agent");
  const input = await $("textarea[aria-label='Agent 消息']");
  await input.waitForDisplayed({ timeout: 60_000 });
  const request = "帮我优化一下最近写作项目的最新章节正文，保持剧情事实与人物设定一致。";
  await input.setValue(request);
  await (await $("button[aria-label='发送消息']")).click();
  // 定位结果会触发工作台到正文页的真实用户导航；先等待该行为完成，
  // 再回到工作台读取主 Agent 的最终验收消息。
  await waitForRoute(new RegExp(`/write/${projectId}\\?chapter=1\\&agent=1`), 120_000);
  await browser.url("/agents");
  await waitForBodyText("主 Agent", 60_000);
  await browser.waitUntil(async () => {
    const text = await $("body").getText();
    return text.includes("委派")
      && text.includes("content_optimization")
      && text.includes("已显式审阅正文优化子 Agent")
      && text.includes("正文尚未写回");
  }, { timeout: 120_000, interval: 1_000, timeoutMsg: "主 Agent 未完成委派和验收" });
  const body = await $("body").getText();
  assert.ok(body.includes(request));
  assert.match(body, /打开子线程/);
  await browser.url(`/write/${projectId}?chapter=1&agent=1`);
  await waitForRoute(new RegExp(`/write/${projectId}\\?chapter=1\\&agent=1`), 120_000);
  const domainSelector = "section[aria-label='领域 Agent 控制台']";
  const followupText = "请复核刚才的优化结果，并补充一句简短的交付说明。";
  // WritingDesk polls the Agent snapshot and may replace the console node
  // while the route settles. Resolve the section and its control in the same
  // fresh-DOM attempt instead of holding a stale WebDriver element.
  await browser.waitUntil(async () => {
    try {
      const scopes = await visibleElements(domainSelector);
      const scope = scopes[0];
      if (!scope) return false;
      const followups = await visibleElements("textarea", scope);
      const followup = followups[0];
      if (!followup) return false;
      await followup.setValue(followupText);
      return true;
    } catch {
      return false;
    }
  }, { timeout: 60_000, interval: 250, timeoutMsg: "正文页领域 Agent 输入框未稳定出现" });
  await browser.waitUntil(async () => {
    try {
      const scopes = await visibleElements(domainSelector);
      const scope = scopes[0];
      if (!scope) return false;
      const button = await buttonMatching("发送给领域 Agent", scope, 0);
      await button.click();
      return true;
    } catch {
      return false;
    }
  }, { timeout: 60_000, interval: 250, timeoutMsg: "正文页领域 Agent 发送按钮未稳定出现" });
  await browser.waitUntil(async () => {
    const scopes = await visibleElements(domainSelector);
    const scope = scopes[0];
    if (!scope) return false;
    const text = await scope.getText();
    if (await approvePendingProposal(scope, text)) return false;
    return text.includes("交付说明") && text.includes("Agent 已完成");
  }, { timeout: 480_000, interval: 1_000, timeoutMsg: "正文页领域 Agent follow-up 未完成" });
  await browser.url("/agents");
  await waitForBodyText("打开子线程");
  assert.match(await $("body").getText(), /主 Agent|子 Agent/);
}

async function exportFamily() {
  await browser.url("/");
  await waitForBodyText("创建项目");
  const row = await firstVisibleContaining('.a4n-project-row', "未命名小说项目");
  assert.ok(row);
  await (await row.$("button[aria-label='更多操作']")).click();
  await clickButton("导出项目族");
  await clickButton("包含派生项目", await currentDialog());
  const downloadDir = process.env.A4N_BROWSER_E2E_DOWNLOAD_DIR;
  assert.ok(downloadDir, "浏览器 E2E 缺少下载目录");
  await browser.waitUntil(async () => {
    const files = await readdir(downloadDir);
    return files.some((file) => file.endsWith(".zip") && !file.endsWith(".crdownload"));
  }, { timeout: 120_000, interval: 1_000, timeoutMsg: "项目族导出没有生成 zip" });
  const files = await readdir(downloadDir);
  const zip = files.find((file) => file.endsWith(".zip"));
  assert.ok(zip);
}

describe("A4N Chromium 全项目最终验收", () => {
  it("从真实登录到项目族导出完成完整用户流程", async () => {
    await prepareSession();
    const projectId = await createProject();
    await enterBlueprint(projectId);
    await createProfiles();
    await createOutlines();
    await importChapter();
    await analyzeAndSync(projectId);
    await exerciseDirectorPlan(projectId);
    await searchRag();
    await optimizeWithDomainAgent(projectId);
    await generateImage(projectId);
    const derived = await createDerivedProjects();
    await exerciseCanvas(derived["创建视频项目"]);
    await exerciseMainAgent(projectId);
    await exportFamily();
  }).timeout(1_800_000);
});
