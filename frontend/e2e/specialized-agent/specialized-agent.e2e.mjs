import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

// Run with the Chromium config and --spec, or wdio.native.conf.mjs, after the CLI fixture.
const fixture = JSON.parse(readFileSync(process.env.A4N_SPECIALIZED_FIXTURE, "utf8"));
const artifacts = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
const native = process.env.A4N_SPECIALIZED_NATIVE === "1";
const cliDirectory = mkdtempSync(join(tmpdir(), "a4n-agent-browser-"));
const cliEnv = { ...process.env };
delete cliEnv.A4N_ACCESS_TOKEN;
const checks = [];
const createdRuns = [];
const llmConfigName = `A4N specialized-agent E2E ${Date.now()}`;

function cli(parts, input) {
  const result = spawnSync(process.env.A4N_CLI || resolve("../target/debug/a4n"), [
    "--output", "agent-json", "--no-input", "--server", process.env.A4N_SPECIALIZED_SERVER,
    "--config", join(cliDirectory, "config.json"), "--context", join(cliDirectory, "context.json"),
    ...parts,
  ], { encoding: "utf8", timeout: 30_000, env: cliEnv, input, maxBuffer: 8 * 1024 * 1024 });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const envelope = JSON.parse(result.stdout);
  assert.equal(envelope.ok, true);
  return envelope.data;
}

async function nativeApi(module, name, args) {
  const result = await browser.executeAsync((module, name, args, done) => {
    import(module).then((api) => api[name](...args))
      .then((value) => done({ value }), (error) => done({ error: String(error) }));
  }, module, name, args);
  assert.equal(result.error, undefined);
  return result.value;
}

const snapshot = async () => native
  ? nativeApi("/src/api/agent.ts", "getAgentThread", [fixture.thread_id])
  : cli(["agent", "thread", "show", fixture.thread_id]);
const savedPlan = async () => (native
  ? await nativeApi("/src/api/chapterVisualPlan.ts", "getChapterVisualPlan", [fixture.project_id, fixture.chapter_id])
  : cli(["call", "get_chapter_visual_plan", "--args", JSON.stringify({
    input: { project_id: fixture.project_id, source_chapter_id: fixture.chapter_id },
  })])).draft;

async function button(text, scope = null) {
  let found;
  await browser.waitUntil(async () => {
    for (const item of await (scope || browser).$$("button")) {
      if (!(await item.isDisplayed())) continue;
      const label = `${await item.getText()} ${await item.getAttribute("aria-label") || ""}`;
      if (typeof text === "string" ? label.includes(text) : text.test(label)) { found = item; return true; }
    }
    return false;
  }, { timeout: 30_000, interval: 100, timeoutMsg: `找不到按钮：${text}` });
  return found;
}

async function click(text, scope = null) {
  const item = await button(text, scope);
  await item.waitForClickable();
  await item.click();
}

async function field(label, tag = "input", scope = null) {
  let found;
  await browser.waitUntil(async () => {
    for (const item of await (scope || browser).$$("label")) {
      const text = await browser.execute((node) => node.textContent, item);
      if (!text.includes(label)) continue;
      const id = await item.getAttribute("for");
      const control = id ? await browser.$(`${tag}[id=${JSON.stringify(id)}]`) : await item.$(tag);
      if (await control.isDisplayed()) {
        found = control;
        return true;
      }
    }
    return false;
  }, { timeout: 5_000, interval: 50, timeoutMsg: `找不到字段：${label}` });
  return found;
}

async function openAgent() {
  if (await $(".director-mobile-inspector").isDisplayed()) await browser.keys("Escape");
  await click("导演 Agent", await $(".director-tool-rail"));
  const console = await $("section[aria-label='领域 Agent 控制台']");
  await console.waitForDisplayed();
  await (await console.$("textarea")).waitForEnabled();
  return console;
}

async function closeAgent() {
  const panel = await $("section[aria-label='领域 Agent 控制台']");
  if (await $(".director-agent-inspector").isDisplayed()) await panel.$('[aria-label="关闭"]').click();
  else await browser.keys("Escape");
  await browser.waitUntil(async () => !(await $("section[aria-label='领域 Agent 控制台']").isDisplayed()), {
    timeout: 5_000, interval: 50, timeoutMsg: "Agent 弹窗未关闭",
  });
  assert.equal(await $(".director-workspace").isExisting(), true, "关闭面板不能关闭编辑器工作区");
}

async function send(task) {
  const previous = new Set((await snapshot()).runs.map((run) => run.id));
  const console = await openAgent();
  await console.$('[aria-label="执行模式"]').click();
  const reviewOption = await $('[role="option"][data-option-index="1"]');
  assert.equal(await reviewOption.getText(), "写入前审阅");
  await reviewOption.click();
  await (await console.$("textarea")).setValue(`A4N_SPECIALIZED ${JSON.stringify(task)}`);
  await click("发送给领域 Agent", console);
  let run;
  await browser.waitUntil(async () => {
    run = (await snapshot()).runs.find((candidate) => !previous.has(candidate.id));
    return Boolean(run);
  }, { timeout: 15_000, interval: 100 });
  createdRuns.push(run.id);
  await closeAgent();
  return run.id;
}

async function waitRun(runId, status, reason) {
  let state;
  await browser.waitUntil(async () => {
    state = await snapshot();
    const run = state.runs.find((item) => item.id === runId);
    assert.ok(run && !["failed", "cancelled", "recovery_required"].includes(run.status), JSON.stringify(run));
    return run.status === status && (!reason || run.waiting_reason === reason);
  }, { timeout: 60_000, interval: 200, timeoutMsg: `Run ${runId} 未进入 ${status}/${reason || ""}` });
  return state;
}

async function approve(runId, command) {
  const state = await waitRun(runId, "waiting", "approval");
  const approval = state.approvals.find((item) => item.status === "pending"
    && state.executions.some((execution) => execution.id === item.execution_id && execution.run_id === runId));
  assert.equal(approval?.command, command);
  const console = await openAgent();
  await click("批准", console);
  await closeAgent();
}

async function openDirector() {
  await browser.url(`${process.env.A4N_BROWSER_E2E_BASE_URL}/video-project/${fixture.project_id}`);
  await click("导演编排");
  await $("section.director-workspace").waitForDisplayed({ timeout: 60_000 });
}

async function capture(width, suffix) {
  await browser.setWindowSize(width, 1000);
  const overflow = await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  assert.equal(overflow, 0, `${width}px 横向溢出`);
  await browser.saveScreenshot(`${artifacts}/director-${width}-${suffix}.png`);
  checks.push(`${width}px ${suffix}: horizontal overflow = 0`);
}

describe("工具驱动导演 Agent 的真实编辑器联动", () => {
  after(async () => {
    if (!createdRuns.length) return;
    for (const run of (await snapshot()).runs) {
      if (createdRuns.includes(run.id) && ["queued", "running", "waiting", "paused", "pausing"].includes(run.status)) {
        if (native) await nativeApi("/src/api/agent.ts", "controlAgentRun", [{ run_id: run.id, thread_id: fixture.thread_id, action: "cancel" }]);
        else cli(["agent", "run", run.id, "cancel", "--thread-id", fixture.thread_id]);
      }
    }
  });
  it("保留来源、后台连接和本地未提交输入", async () => {
    assert.equal(fixture.success, true, "先完成 CLI fixture");
    if (!native) {
      cli(["auth", "login", "--username", process.env.A4N_BROWSER_E2E_USERNAME, "--password-stdin"], `${process.env.A4N_BROWSER_E2E_PASSWORD}\n`);
      cli(["capabilities", "describe", "get_chapter_visual_plan"]);
    }
    if (native) {
      await browser.waitUntil(async () => (await browser.getUrl()).startsWith(process.env.A4N_BROWSER_E2E_BASE_URL));
    } else {
      await browser.url(process.env.A4N_BROWSER_E2E_BASE_URL);
    }
    await $("input[autocomplete='username']").waitForDisplayed();
    await $("input[autocomplete='username']").setValue(process.env.A4N_BROWSER_E2E_USERNAME);
    await $("input[type='password']").setValue(process.env.A4N_BROWSER_E2E_PASSWORD);
    await $("form button[type='submit']").click();
    await button(/创建项目|Create project/);
    await click(/展开快捷工具|Open quick tools/);
    await click(/界面语言|Interface language/);
    await click(/中文|Chinese/);
    await button("创建项目");
    await browser.url(`${process.env.A4N_BROWSER_E2E_BASE_URL}/settings`);
    await click("LLM 服务");
    await click("新增 LLM 配置");
    await (await field("配置名称")).setValue(llmConfigName);
    await (await field("模型")).setValue("a4n-virtual-text");
    await (await field("接口地址")).setValue(process.env.A4N_BROWSER_E2E_PROVIDER_URL);
    const key = await field("访问密钥");
    await key.setValue("a4n-e2e-provider-key");
    await click(/创建|保存/);
    await click(llmConfigName);
    const configured = (await nativeApi("/src/api/config/llm.ts", "listLlmConfig", [])).find((config) => config.name === llmConfigName);
    assert.ok(configured, "刚保存的测试模型配置必须可读");
    await nativeApi("/src/api/config/llm.ts", "setDefaultLlmConfig", [configured.id]);
    assert.equal((await nativeApi("/src/api/config/llm.ts", "listLlmConfig", [])).find((config) => config.id === configured.id).is_default, true);
    await openDirector();
    const firstTab = await browser.getWindowHandle();
    const before = await savedPlan();
    const prompt = `原视图的镜头描述 ${Date.now()}`;
    const runId = await send({ mode: "editor", save_editor: true, prompt });
    await (await $$("button.director-segment"))[1].click();
    await approve(runId, "save_editor_draft");
    await approve(runId, "edit_chapter_visual_plan_draft");
    const completed = await waitRun(runId, "completed");
    const after = await savedPlan();
    const expected = structuredClone(before.plan);
    expected.segments[0].prompt = prompt;
    assert.deepEqual(after.plan, expected);
    assert.equal(after.record.revision, before.record.revision + 1);
    const requests = completed.events.filter((event) => event.run_id === runId && event.editor_request).map((event) => event.editor_request);
    assert.ok(requests.length >= 5);
    assert.ok(requests.every((request) => request.source.shot_id === "shot-1"));
    assert.match(await $(".director-segment.is-selected").getAttribute("aria-label"), /用户并发改名/);
    await (await $$("button.director-segment"))[0].click();
    await browser.waitUntil(async () => (await (await field("变化描述", "textarea")).getValue()) === prompt, { timeout: 15_000, timeoutMsg: "Agent 修改没有自动应用到编辑器" });
    await (await $$("button.director-segment"))[1].click();
    checks.push("关闭弹窗后完成 inspect/read/save/inspect/edit；发送后切选区仍只改原镜头，选区保持");
    const controlsRun = await send({ mode: "view_controls", actions: [
      { kind: "seek", at_ms: 500 }, { kind: "select", object_kind: "segment", object_id: "shot-2" }, { kind: "fit" },
    ] });
    const controlled = await waitRun(controlsRun, "completed");
    assert.equal(controlled.executions.filter((execution) => execution.run_id === controlsRun && execution.tool_name === "control_editor_view" && execution.status === "succeeded").length, 3);
    assert.equal(await $('[aria-label="播放位置"]').getValue(), "500");
    assert.deepEqual(await savedPlan(), after);
    checks.push("面板关闭时仍可按明确指令定位播放头、选择镜头和适配视图，不修改数据库方案");
    if (native) {
      assert.equal(await nativeApi("/src/utils/runtimeMode.ts", "isTauriRuntime", []), true);
      assert.equal(await nativeApi("/src/utils/runtimeMode.ts", "shouldUseHttpBackend", []), true);
      await browser.saveScreenshot(`${artifacts}/director-native.png`);
      checks.push("真实 Tauri 窗口通过已配置的 HTTP/SSE 和共享 Host 完成原生编辑器联动");
      writeFileSync(`${artifacts}/specialized-agent-native-report.json`, JSON.stringify({ success: true, checks, runId }, null, 2), { mode: 0o600 });
      return;
    }

    await browser.newWindow(await browser.getUrl());
    const secondTab = await browser.getWindowHandle();
    await openDirector();
    await (await $$("button.director-segment"))[1].click();
    const secondRun = await send({ mode: "inspect_only" });
    const second = await waitRun(secondRun, "completed");
    const secondSource = second.events.find((event) => event.run_id === secondRun && event.editor_request).editor_request.source;
    assert.equal(secondSource.shot_id, "shot-2");
    assert.notEqual(secondSource.view_instance_id, requests[0].source.view_instance_id);
    checks.push("同一 canonical thread 的两个标签页按各自 Turn 绑定来源");

    await browser.switchToWindow(firstTab);
    const blockedRun = await send({ mode: "editor", save_editor: true, prompt: "不得覆盖未提交弹窗" });
    await waitRun(blockedRun, "waiting", "approval");
    await (await $$("button.director-segment"))[0].click();
    await click("与后一段合并");
    const draftField = await field("合并后的变化描述", "textarea");
    await draftField.setValue("仍在输入、尚未提交的描述");
    await browser.switchToWindow(secondTab);
    await approve(blockedRun, "save_editor_draft");
    const blocked = await waitRun(blockedRun, "completed");
    assert.ok(blocked.executions.some((execution) => execution.run_id === blockedRun && execution.status === "failed" && /拆分|合并/.test(execution.error || "")));
    assert.deepEqual(await savedPlan(), after);
    await browser.switchToWindow(firstTab);
    assert.equal(await (await field("合并后的变化描述", "textarea")).getValue(), "仍在输入、尚未提交的描述");
    checks.push("另一标签页批准不会覆盖原视图的未提交弹窗；失败保持原文案和保存版本");
    await browser.keys("Escape");
    await click("放弃编辑");
    await $("h3=合并镜头").waitForDisplayed({ reverse: true });
    const backgroundPrompt = `原窗口关闭后继续 ${Date.now()}`;
    const backgroundRun = await send({ mode: "edit", project_id: fixture.project_id, chapter_id: fixture.chapter_id, shot_id: "shot-1", prompt: backgroundPrompt });
    await browser.closeWindow();
    await browser.switchToWindow(secondTab);
    await approve(backgroundRun, "edit_chapter_visual_plan_draft");
    const background = await waitRun(backgroundRun, "completed");
    assert.equal(background.events.filter((event) => event.run_id === backgroundRun && event.editor_request).length, 0);
    assert.equal((await savedPlan()).plan.segments[0].prompt, backgroundPrompt);
    assert.match(await $(".director-segment.is-selected").getAttribute("aria-label"), /用户并发改名/);
    await (await $$("button.director-segment"))[0].click();
    await browser.waitUntil(async () => (await (await field("变化描述", "textarea")).getValue()) === backgroundPrompt);
    checks.push("关闭原窗口后，显式已保存版本的编辑仍完成；另一窗口无需刷新即可收到结果");

    for (const width of [1440, 1024, 390]) {
      await capture(width, "page");
      await openAgent();
      await capture(width, "agent");
      await $("section[aria-label='领域 Agent 控制台'] textarea").click();
      await browser.keys("Tab");
      if (width === 390) assert.equal(await browser.execute(() => Boolean(document.activeElement?.closest('[role="dialog"]'))), true);
      await closeAgent();
    }
    checks.push("移动端弹窗保持键盘焦点，桌面侧栏使用页面 Tab 顺序；关闭面板后工作区连接保留");
    writeFileSync(`${artifacts}/specialized-agent-report.json`, JSON.stringify({ success: true, checks, runId, controlsRun, secondRun, blockedRun, backgroundRun }, null, 2), { mode: 0o600 });
  }).timeout(600_000);
});
