import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import {
  prepareSession, captureBrowserDiagnostics, visibleElements, firstVisibleContaining, buttonMatching, clickButton,
  currentDialog, fieldByLabel, setField, waitForBodyText, waitForBodyMatch,
  waitForRoute, waitForNoDialog, closeDialog, waitAndCloseProgress, tab, sectionContaining,
} from "./user-flow.mjs";

const stages = ["inspiration", "blueprint", "outline", "chapter", "analysis", "director", "shooting", "segments", "media"];
const requested = (process.env.A4N_CREATION_E2E_STAGES ?? stages.join(",")).split(",");
assert.ok(requested.length && requested.every((stage) => stages.includes(stage)), "unknown creation stage");
const lastStage = Math.max(...requested.map((stage) => stages.indexOf(stage)));
const report = Object.fromEntries(stages.slice(0, lastStage + 1).map((stage) => [stage, { status: "not_run" }]));
const artifact = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
let projectId;
let videoId;
const firstIdea = "主角林岚必须保留旧怀表；创作六章都市时间悬疑，背景是旧车站。";
const outlineMarker = "用户修改章纲：林岚在门口检查旧怀表。";
const bodyMarker = "作者补充：林岚在门口停顿片刻。";

function saveReport() {
  writeFileSync(`${artifact}/stages.json`, JSON.stringify(report, null, 2));
}

async function stage(name, run) {
  report[name] = { status: "running" };
  saveReport();
  try {
    await run();
    assert.deepEqual(await captureBrowserDiagnostics(`${artifact}/${name}`), [], `${name}: 浏览器异常或服务端错误`);
    await browser.saveScreenshot(`${artifact}/${name}.png`);
    report[name] = { status: "passed", projectId, videoId };
  } catch (error) {
    report[name] = { status: "failed", error: String(error), projectId, videoId };
    throw error;
  } finally {
    saveReport();
  }
}

async function refreshAndRead(text, tabs = []) {
  await browser.refresh();
  for (const label of tabs) await tab(label);
  await waitForBodyText(text);
}

async function inspiration() {
  await clickButton("创建项目");
  await clickButton("AI 辅助创作", await currentDialog());
  await waitForRoute(/\/inspiration\/\d+/);
  projectId = Number((await browser.getUrl()).match(/\/inspiration\/(\d+)/)[1]);
  const input = await $('[aria-label="输入灵感对话内容"]');
  await input.waitForDisplayed();
  assert.equal(await (await buttonMatching(/^发送\b|^发送\s/)).isEnabled(), false);
  await input.setValue(firstIdea);
  await clickButton(/^发送/);
  await waitForBodyText("先选择故事方向");
  const choices = await visibleElements("button.inspiration-option-card");
  assert.equal(choices.length, 3);
  await choices[0].click();
  await waitForBodyText("林岚为什么进入钟楼");
  await $('[aria-label="输入灵感对话内容"]').setValue("林岚进入钟楼寻找未来来信的来源，保留旧怀表这一约束。");
  await clickButton(/^发送/);
  await waitForBodyText("素材已经足够完整");
  await refreshAndRead(firstIdea);
  await waitForBodyText("素材已经足够完整");
  assert.equal(await (await buttonMatching("生成首版蓝图")).isEnabled(), true);
  for (const option of await visibleElements("button.inspiration-option-card")) {
    assert.equal(await option.isEnabled(), false, "old options must not submit an earlier round again");
  }
}

async function blueprint() {
  await clickButton("生成首版蓝图");
  await waitAndCloseProgress("生成首版蓝图");
  await waitForBodyText("确认进入项目");
  assert.match(await $("body").getText(), /林岚|旧车站/);
  await clickButton("调整蓝图");
  const dialog = await currentDialog();
  await setField("调整蓝图", "保留旧怀表和主角动机，强化空间连续性。", "textarea", dialog);
  await clickButton("提交优化", dialog);
  await waitAndCloseProgress("调整蓝图");
  await waitForBodyText("优化后，林岚更早主动选择相信周衡");
  await clickButton("确认进入项目");
  await waitForRoute(new RegExp(`/novel/${projectId}`));
  await refreshAndRead("优化后，林岚更早主动选择相信周衡");
  await tab("角色档案");
  await waitForBodyText("林岚");
  await browser.url(`/novel/${projectId}`);
}

async function outline() {
  await tab("大纲");
  await tab("分部大纲");
  let section = await sectionContaining("分部大纲");
  await setField("生成部数", "2", "input", section);
  await setField("每部章节数", "3", "input", section);
  await clickButton("生成分部", section);
  await waitAndCloseProgress("生成分部大纲");
  await waitForBodyText("来信与失去的七分钟");
  await clickButton("第 1 部", await sectionContaining("分部大纲"));
  let dialog = await currentDialog();
  await clickButton("编辑正式分部", dialog);
  dialog = await currentDialog();
  await setField("分部摘要", "用户修改分部：林岚追查七分钟偏差，保留旧怀表和机房入口的线索。", "textarea", dialog);
  await clickButton("保存正式分部", dialog);
  await waitForNoDialog();
  await refreshAndRead("用户修改分部", ["大纲", "分部大纲"]);
  await tab("章节大纲");
  await setField("本次生成章数", "3", "input");
  await clickButton("生成章纲");
  await waitAndCloseProgress("生成章节大纲");
  await waitForBodyText("雨夜来信");
  await (await visibleElements('[aria-label="更多章纲操作"]'))[0].click();
  await clickButton("编辑章纲", await currentDialog());
  await waitForNoDialog();
  await setField("章节摘要：场景、动机、悬念", outlineMarker, "textarea");
  await clickButton(/^保存\s/, await sectionContaining("章节大纲"));
  await refreshAndRead(outlineMarker, ["大纲", "章节大纲"]);
}

async function chapter() {
  await browser.url(`/write/${projectId}?chapter=1`);
  await waitForBodyText("正文");
  await clickButton("写作助手");
  const assistant = await $('aside[aria-label="写作助手"]');
  await assistant.waitForDisplayed();
  const length = await firstVisibleContaining("summary", "正文长度", assistant);
  assert.ok(length);
  await length.click();
  await setField("最少字数", "800", "input", assistant);
  await setField("最多字数", "1600", "input", assistant);
  await clickButton("开始生成", assistant);
  await clickButton(/^生成\s/, await currentDialog());
  await browser.waitUntil(async () => {
    const busy = await visibleElements('.a4n-stream-control[aria-busy="true"]');
    const editors = await visibleElements('textarea[aria-label="正文"], textarea[aria-label="章节正文"]');
    return !busy.length && editors.length && (await editors[0].getValue()).includes("林岚");
  }, { timeout: 180_000, interval: 500, timeoutMsg: "正文生成未完成或编辑器没有结果" });
  await browser.refresh();
  const editor = await $('textarea[aria-label="正文"], textarea[aria-label="章节正文"]');
  await editor.waitForDisplayed();
  const body = await editor.getValue();
  assert.ok(body.length > 500 && body.includes("旧怀表"));
  await editor.setValue(`${body}\n\n${bodyMarker}`);
  await clickButton(/^保存/);
  await waitForBodyMatch(/已保存|保存成功/);
  await browser.refresh();
  await editor.waitForDisplayed();
  assert.equal(await editor.getValue(), `${body}\n\n${bodyMarker}`);
  await browser.setWindowSize(390, 844);
  const narrowBody = `${body}\n\n${bodyMarker}\n\n窄屏补充：保留怀表。`;
  await editor.setValue(narrowBody);
  await clickButton("保存修改");
  await waitForBodyMatch(/已保存|保存成功/);
  await browser.refresh();
  await editor.waitForDisplayed();
  assert.equal(await editor.getValue(), narrowBody);
  await browser.saveScreenshot(`${artifact}/chapter-narrow.png`);
  await browser.setWindowSize(1440, 1000);
}

async function analysis() {
  await tab("分析");
  await clickButton(/生成分析|重新分析|继续分析/);
  await waitAndCloseProgress("任务已完成");
  const selector = 'section[aria-label="本章资料同步状态"]';
  await $(selector).waitForDisplayed();
  await browser.waitUntil(async () => {
    const statuses = await (await $(selector)).$$("dd");
    return statuses.length === 5 && (await statuses.map((item) => item.getText())).every((text) => text.trim() === "已完成");
  }, { timeout: 180_000, interval: 500, timeoutMsg: "分析产物或资料同步未完成" });
  await browser.refresh();
  await tab("分析");
  await $(selector).waitForDisplayed();
  const statuses = await (await $(selector)).$$("dd");
  assert.equal(statuses.length, 5);
  for (const item of statuses) assert.equal((await item.getText()).trim(), "已完成");
  await tab("正文");
  assert.ok((await $('textarea[aria-label="正文"], textarea[aria-label="章节正文"]').getValue()).includes(bodyMarker));
}

async function director() {
  await browser.url("/");
  await waitForBodyText("创建项目");
  const row = await $(`[data-project-family="${projectId}"] .a4n-project-row`);
  await row.waitForDisplayed({ timeout: 60_000 });
  await row.$('button[aria-label="更多操作"]').click();
  await clickButton("创建视频项目");
  await clickButton("下一步", await currentDialog());
  const dialog = await currentDialog();
  await setField("章节范围", "1-1", "input", dialog);
  await clickButton("创建", dialog);
  await waitForRoute(/\/video-project\/\d+/);
  videoId = Number((await browser.getUrl()).match(/\/video-project\/(\d+)/)[1]);
  await tab("导演编排");
  await $("section.director-workspace").waitForDisplayed();
  assert.equal((await $$(".director-segment")).length, 1);
  await $(".director-segment").click();
  let inspector = await $(".director-inspector");
  await setField("镜头名称", "用户导演镜头", "input", inspector);
  await setField("变化描述", "林岚在旧车站观察怀表，沿封闭通道走向机房入口。", "textarea", inspector);
  await inspector.$('button[role="combobox"][aria-label="场景"]').click();
  const scene = await firstVisibleContaining('[role="option"]', "旧车站站厅");
  if (scene) {
    await scene.click();
  } else {
    const createScene = await firstVisibleContaining('[role="option"]', "新建自定义场景");
    assert.ok(createScene, "video project must allow creating a local scene");
    await createScene.click();
    await setField("自定义场景名称", "旧车站站厅", "input", inspector);
    await setField("场景描述", "旧车站站厅，林岚沿封闭通道走向机房入口。", "textarea", inspector);
    await clickButton("保存并绑定", inspector);
  }
  await clickButton("调整整体时长");
  await setField("整体时长", "6", "input", await currentDialog());
  await browser.keys("Enter");
  await clickButton("完成", await currentDialog());
  await waitForNoDialog();
  await clickButton("保存草稿");
  await waitForBodyText("草稿已保存");
  await browser.refresh();
  await $(".director-segment").waitForDisplayed();
  await $(".director-segment").click();
  inspector = await $(".director-inspector");
  assert.equal(await (await fieldByLabel("镜头名称", "input", inspector)).getValue(), "用户导演镜头");
  const scrubber = await $('[aria-label="播放位置"]');
  assert.equal(await scrubber.getAttribute("max"), "6000", "saved timeline duration must survive reload");
  await scrubber.click();
  await browser.keys("Home");
  await browser.keys("ArrowRight");
  assert.equal(await scrubber.getValue(), "1", "native keyboard input must update the playhead");
  await clickButton("确认视觉编排");
  await waitForBodyText("方案已确认");
  await refreshAndRead("方案已确认");
}

async function shooting() {
  await browser.url(`/video-project/${videoId}`);
  await tab("拍摄剧本");
  await clickButton("生成第一个场景");
  await waitAndCloseProgress("拍摄");
  await waitForBodyText("进入编辑");
  await clickButton("进入编辑");
  const details = await firstVisibleContaining("summary", "场景");
  assert.ok(details);
  await details.click();
  await setField("场景标题", "用户编辑的拍摄场景", "textarea");
  await clickButton("保存改编");
  await waitForBodyMatch(/保存成功|已保存|改编已保存/);
  await refreshAndRead("用户编辑的拍摄场景", ["拍摄剧本"]);
}

async function segments() {
  await browser.url(`/video-project/${videoId}`);
  await tab("片段规划");
  const panel = await $('section[aria-label="视频片段规划"]');
  await panel.waitForDisplayed();
  await waitForBodyText("用户编辑的拍摄场景");
  await clickButton("规划此场景", panel);
  await waitAndCloseProgress("片段规划");
  await clickButton("编辑规划", panel);
  const details = await firstVisibleContaining("summary", "片段", panel);
  assert.ok(details);
  await details.click();
  await setField("场景摘要", "用户片段约束：怀表留在林岚手中。", "textarea", panel);
  await clickButton("保存规划", panel);
  await waitForBodyMatch(/已保存|保存成功/);
  await refreshAndRead("用户片段约束：怀表留在林岚手中。");
}

async function media() {
  await tab("片段生成");
  await $('[data-video-segment="1"]').click();
  await clickButton("首帧续接", await currentDialog());
  await clickButton("配置首帧", await currentDialog());
  await waitForBodyText("首帧配置");
  await clickButton("准备首帧提示词", await currentDialog());
  await waitAndCloseProgress("首帧提示词");
  await clickButton("配置首帧", await currentDialog());
  await waitForBodyText("首帧配置");
  await clickButton("生成图片", await currentDialog());
  await waitForBodyText("将调用图片模型");
  await clickButton("生成图片", await currentDialog());
  await waitAndCloseProgress("生成首帧图片");
  const frame = await $('button[aria-label="配置首帧"] img');
  await frame.waitForDisplayed();
  await browser.waitUntil(async () => browser.execute((element) => element.complete && element.naturalWidth > 0, frame), { timeout: 30_000 });
  await clickButton("生成最终提示词", await currentDialog());
  await waitAndCloseProgress("视频提示词");
  await clickButton(/^生成视频/, await currentDialog());
  await waitAndCloseProgress("生成视频");
  await closeDialog();
  await clickButton(/^采用\s/);
  await waitForBodyText("已采用片段 1 的当前视频");
  await browser.refresh();
  await tab("片段生成");
  await waitForBodyText("已采用成片");
  const player = await $("video");
  await player.waitForExist({ timeout: 120_000 });
  await browser.waitUntil(async () => browser.execute((element) => element.readyState >= 1 && element.duration > 0, player), { timeout: 60_000 });
  const media = await browser.execute((element) => ({ duration: element.duration, width: element.videoWidth, height: element.videoHeight }), player);
  assert.ok(Math.abs(media.duration - 6) < 0.1 && media.width > 0 && media.height > 0, "persisted video must decode with the planned duration after reload");
  await browser.execute(async (element) => { element.muted = true; await element.play(); }, player);
  await browser.waitUntil(async () => browser.execute((element) => element.ended && !element.error, player), { timeout: 20_000, timeoutMsg: "已保存的视频未能完整播放" });
}

describe("创作阶段的真实用户闭环", () => {
  it("从灵感输入到生成、编辑、刷新重读及下游消费", async () => {
    saveReport();
    await prepareSession();
    const flows = { inspiration, blueprint, outline, chapter, analysis, director, shooting, segments, media };
    for (const name of stages.slice(0, lastStage + 1)) {
      await stage(name, flows[name]);
    }
  });
});
