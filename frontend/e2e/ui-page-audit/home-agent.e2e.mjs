import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { buildPageFixtures, seedLocalModels } from "./fixtures.mjs";
import { homeAgentFixtures } from "./home-agent-fixtures.mjs";

const output = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/home-agent/", import.meta.url));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";
assert.match(phase, /^[a-z0-9_-]+$/i);
const widths = [1440, 900, 390];
const height = 1000;
const timeout = 6000;
const report = [];
let fixtures, current, auditWindow;
let navigation = 0;
const mainSeed = homeAgentFixtures.agentSnapshots[0];
const budgetSeed = homeAgentFixtures.agentSnapshots[1];
const activityDraft = "保留这条草稿，查看审批后继续讨论。";

const literal = (value) => value.includes('"')
  ? "concat(" + value.split('"').map((part) => JSON.stringify(part)).join(",'\"',") + ")"
  : JSON.stringify(value);
async function button(label, scope) {
  let result;
  const selector = (scope ? ".//" : "//") + "button[@aria-label=" + literal(label) + " or normalize-space(.)=" + literal(label) + "]";
  await browser.waitUntil(async () => {
    for (const candidate of await (scope ? scope.$$(selector) : $$(selector))) {
      if (await candidate.isDisplayed()) { result = candidate; return true; }
    }
    return false;
  }, { timeout, timeoutMsg: "No visible button: " + label });
  return result;
}

// Native wheel events preserve the distinction between a scrollable region and clipped content.
async function reach(element, end = false) {
  await element.waitForExist({ timeout });
  let state;
  let wheelSteps = 0;
  for (let attempt = 0; attempt < 26; attempt++) {
    state = await element.execute((node, atEnd) => {
      const r = node.getBoundingClientRect(), clip = { left: 0, right: innerWidth, top: 0, bottom: innerHeight };
      for (let parent = node.parentElement; parent; parent = parent.parentElement) {
        const style = getComputedStyle(parent), p = parent.getBoundingClientRect();
        if (/(auto|scroll|hidden|clip)/.test(style.overflowX)) { clip.left = Math.max(clip.left, p.left); clip.right = Math.min(clip.right, p.right); }
        if (/(auto|scroll|hidden|clip)/.test(style.overflowY)) { clip.top = Math.max(clip.top, p.top); clip.bottom = Math.min(clip.bottom, p.bottom); }
      }
      const x = atEnd ? r.right - Math.min(6, r.width / 2) : r.x + r.width / 2;
      const y = atEnd ? r.bottom - Math.min(6, r.height / 2) : r.y + r.height / 2;
      const inside = x > clip.left && x < clip.right && y > clip.top && y < clip.bottom;
      const hit = inside ? document.elementFromPoint(x, y) : null;
      const reached = Boolean(hit && (hit === node || node.contains(hit)));
      const axis = x <= clip.left || x >= clip.right ? "x" : y <= clip.top || y >= clip.bottom ? "y" : null;
      let surface = null;
      for (let parent = node.parentElement; parent && axis; parent = parent.parentElement) {
        const style = getComputedStyle(parent), r = parent.getBoundingClientRect();
        const overflow = axis === "x" ? style.overflowX : style.overflowY;
        const extent = axis === "x" ? parent.scrollWidth - parent.clientWidth : parent.scrollHeight - parent.clientHeight;
        const left = Math.max(1, r.left), right = Math.min(innerWidth - 1, r.right);
        const top = Math.max(1, r.top), bottom = Math.min(innerHeight - 1, r.bottom);
        if (/(auto|scroll)/.test(overflow) && extent > 1 && right - left > 16 && bottom - top > 16) {
          surface = { x: (left + right) / 2, y: (top + bottom) / 2, span: axis === "x" ? right - left : bottom - top };
          break;
        }
      }
      const distance = surface ? axis === "x" ? x - surface.x : y - surface.y : 0;
      return {
        reached, axis, surface, clip,
        rect: { x: r.x, y: r.y, width: r.width, height: r.height },
        label: node.getAttribute("aria-label") || node.innerText?.trim().slice(0, 100),
        delta: surface ? Math.round(Math.max(-surface.span / 2, Math.min(surface.span / 2, distance))) : 0,
      };
    }, end);
    if (state.reached) return { ...state, wheelSteps };
    if (!state.axis) { await browser.pause(100); continue; }
    if (!state.surface || !state.delta) break;
    await browser.performActions([{ type: "wheel", id: "home-agent-wheel", actions: [{
      type: "scroll", origin: "viewport", x: Math.round(state.surface.x), y: Math.round(state.surface.y),
      deltaX: state.axis === "x" ? state.delta : 0, deltaY: state.axis === "y" ? state.delta : 0, duration: 120,
    }] }]);
    wheelSteps++;
    await browser.releaseActions();
    await browser.pause(100);
  }
  return { ...state, wheelSteps };
}
async function press(element) {
  await element.waitForDisplayed({ timeout });
  const position = await reach(element);
  assert.ok(position.reached, "Control is clipped or obscured: " + JSON.stringify(position));
  assert.ok(await element.isEnabled(), "Control is disabled: " + position.label);
  assert.notEqual(await element.getAttribute("aria-disabled"), "true");
  await element.click();
  return element;
}
const click = async (label, scope) => press(await button(label, scope));
async function menu(label) {
  return press(await $('//*[@role="menuitem" and contains(normalize-space(.),' + literal(label) + ')]'));
}
async function dialog(title) {
  let result;
  await browser.waitUntil(async () => {
    for (const candidate of await $$('.a4n-modal-layer[data-modal-state="entering"] [role="dialog"]')) {
      const label = await candidate.execute((node) => document.getElementById(node.getAttribute("aria-labelledby"))?.textContent.trim());
      if (label === title && await candidate.isDisplayed()) { result = candidate; return true; }
    }
    return false;
  }, { timeout, timeoutMsg: "No active dialog: " + title });
  return result;
}
async function choose(label, option, scope) {
  const trigger = await scope.$('button[role="combobox"][aria-label=' + JSON.stringify(label) + ']');
  await trigger.waitForDisplayed({ timeout });
  if ((await trigger.getText()).trim() === option) return;
  await press(trigger);
  await press(await $('//*[@role="option" and normalize-space(.)=' + literal(option) + ']'));
}
const projectTitles = (scope) => scope.execute((node) => [...node.querySelectorAll(".a4n-project-row h3")].map((heading) => heading.textContent.trim()).sort());
async function expectProjects(scope, records) {
  const expected = records.map((record) => record.title).sort();
  await browser.waitUntil(async () => JSON.stringify(await projectTitles(scope)) === JSON.stringify(expected), { timeout, timeoutMsg: "Filtered project rows do not match the requested fixture records" });
  return { projectIds: records.map((record) => record.id), titles: expected };
}
async function home() {
  await browser.switchToWindow(auditWindow);
  await browser.url("/?home-agent-reset=" + (++navigation));
  const scope = await $(".a4n-home-content");
  await scope.waitForDisplayed({ timeout });
  await choose("项目类型", "全部", scope);
  await choose("访问权限", "全部可访问", scope);
  await click("最近项目", scope);
  await expectProjects(scope, fixtures.projectRecords);
  return scope;
}
async function agent(threadId = mainSeed.thread.id, activity = false) {
  await home();
  await click("展开快捷工具");
  await click("查看 Agent 工作状态");
  await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/agents", { timeout });
  const scope = await $(".agent-monitor-page");
  await scope.$(".agent-monitor-thread-list").waitForDisplayed({ timeout });
  const expected = fixtures.agentSnapshots.find((snapshot) => snapshot.thread.id === threadId);
  const thread = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)=' + literal(expected.thread.title) + ']]');
  await press(thread);
  await browser.waitUntil(async () => (await scope.$(".agent-monitor-session-header h2").getText()) === expected.thread.title
    && (await scope.$$('[data-agent-message-role]')).length === expected.messages.length, { timeout });
  const toggle = await scope.$(".agent-monitor-activity-toggle");
  if (expected.approvals.some((approval) => approval.status === "pending")) {
    await browser.waitUntil(async () => (await toggle.getAttribute("aria-pressed")) === "true", { timeout });
  }
  if ((await toggle.getAttribute("aria-pressed") === "true") !== activity) await press(toggle);
  if (activity) await scope.$(".agent-monitor-activity-pane").waitForDisplayed({ timeout });
  else await scope.$('[aria-label="Agent 消息"]').waitForDisplayed({ timeout });
  return scope;
}
async function activitySection(title) {
  const scope = await agent(mainSeed.thread.id, true);
  return scope.$('.//div[contains(@class,"agent-monitor-activity-pane")]//section[.//h3[normalize-space(.)=' + literal(title) + ']]');
}
async function budgetDialog() {
  const scope = await agent(budgetSeed.thread.id);
  await click("调整预算并继续", scope);
  const modal = await dialog("调整预算并继续");
  await browser.waitUntil(async () => (await modal.$$("input")).length === 4, { timeout });
  return modal;
}
async function acceptPendingAgentSend() {
  return browser.execute(() => {
    const request = window.__uiAudit.agentMessages.shift();
    if (!request) throw new Error("Expected an Agent message request");
    const accepted = {
      thread_id: request.input.thread_id,
      message_id: `accepted-${request.input.client_message_id}`,
      turn_id: `turn-${request.input.client_message_id}`,
      run_id: `run-${request.input.client_message_id}`,
      status: "queued",
      created_run: true,
    };
    request.resolve(Response.json(accepted));
    return accepted;
  });
}
async function resolvePendingAgentReads(threadIds) {
  await browser.execute((ids) => {
    const audit = window.__uiAudit;
    for (const pending of [...audit.pending]) {
      if (!ids.includes(pending.command) && pending.command !== "agent-threads") continue;
      const result = pending.command === "agent-threads"
        ? audit.fixtures.agentSnapshots.map((item) => item.thread)
        : audit.fixtures.agentSnapshots.find((item) => item.thread.id === pending.command);
      if (!result) throw new Error(`Missing Agent read fixture: ${pending.command}`);
      pending.resolve(Response.json(result));
      audit.pending = audit.pending.filter((item) => item !== pending);
    }
    audit.deferred = audit.deferred.filter((command) =>
      !ids.includes(command) && command !== "agent-threads",
    );
  }, threadIds);
}
async function shot(scope, width, stage, evidence = {}) {
  await browser.pause(200);
  const state = await browser.execute(() => ({
    width: innerWidth, height: innerHeight, url: location.pathname,
    overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors,
  }));
  const file = `${phase}-${current.id}-${width}-${stage}.png`;
  await browser.saveScreenshot(output + file);
  const png = await readFile(output + file);
  report.push({ id: current.id, task: current.task, stage, file, ...state, evidence, text: (await scope.getText()).slice(0, 16000) });
  assert.equal(state.width, width); assert.equal(png.readUInt32BE(16), width);
  assert.equal(state.overflow, 0); assert.deepEqual(state.unexpected, []); assert.deepEqual(state.errors, []);
}
async function matrix(spec) {
  current = { id: spec.id, task: spec.task };
  try {
    for (const width of widths) {
      current.width = width;
      await browser.setWindowSize(width, 1000);
      const scope = await spec.open(width);
      if (spec.text) await browser.waitUntil(async () => (await scope.getText()).includes(spec.text), { timeout, timeoutMsg: "Populated content missing: " + spec.text });
      const evidence = spec.check ? await spec.check(scope, width) : {};
      await shot(scope, width, "initial", evidence);
      if (spec.last) {
        const last = await reach(await spec.last(scope), true);
        await shot(scope, width, "last", { last });
        assert.ok(last.reached, "The last item is not user-scrollable: " + JSON.stringify(last));
      }
      if (spec.after) await spec.after(scope);
    }
  } catch (error) {
    current.error = error.stack ?? String(error);
    throw error;
  }
}

describe("Home and global Agent page audit", function () {
  this.timeout(90000);
  before(async () => {
    await mkdir(output, { recursive: true });
    const base = await buildPageFixtures(browser.options.baseUrl);
    fixtures = { ...base, ...homeAgentFixtures,
      agentSnapshots: [...new Map([...base.agentSnapshots, ...homeAgentFixtures.agentSnapshots].map((snapshot) => [snapshot.thread.id, snapshot])).values()],
    };
    await installUiFixtures(browser, fixtures);
    auditWindow = await browser.getWindowHandle();
    await home();
    await seedLocalModels(browser);
  });
  afterEach(async function () {
    if (this.currentTest.state === "failed") {
      const state = await browser.execute(() => ({ width: innerWidth, url: location.href, text: document.body.innerText, errors: window.__uiAudit?.errors, unexpected: window.__uiAudit?.unexpected }));
      const file = `${phase}-${current?.id ?? "setup"}-${state.width}-failure.png`;
      await browser.saveScreenshot(output + file);
      const failure = { id: current?.id, file, outcome: "failed", error: current?.error ?? this.currentTest.err?.stack, ...state };
      report.push(failure);
      await writeFile(output + file.replace(/\.png$/, ".json"), JSON.stringify(failure, null, 2));
    }
    await writeFile(output + phase + "-report.json", JSON.stringify(report, null, 2));
  });

  it("UX-1779 prevents duplicate sub-agent creation while reporting the pending action", async function () {
    current = { id: "UX-1779-subagent-create-busy", task: "子 Agent 创建期间明确互斥并保留创建表单", visited: [], plannedWidths: [1440] };
    await browser.switchToWindow(auditWindow);
    await browser.url("/agents?subagent-create-" + Date.now());
    const scope = await $(".agent-monitor-page");
    await scope.$(".agent-monitor-thread-list").waitForDisplayed({ timeout });
    const mainThread = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)=' + literal(mainSeed.thread.title) + ']]');
    await press(mainThread);
    await browser.waitUntil(async () => (await scope.$(".agent-monitor-session-header h2").getText()) === mainSeed.thread.title, { timeout });
    await click("新建子 Agent", scope);
    const form = await scope.$(".agent-monitor-create-form");
    await form.$('[aria-label="线程名称（可选）"]').setValue("并发创建路径复核");
    await form.$('[aria-label="子 Agent 能力域"]').setValue("director");
    await browser.execute(() => window.__uiAudit.deferred.push("create_agent_thread"));
    await click("创建", form);
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "create_agent_thread" && item.body.kind === "subagent"),
    ), { timeout, timeoutMsg: "The sub-agent creation request must remain pending" });
    const createButton = await form.$("button.flex-1");
    const closeButton = await form.$("button:last-child");
    assert.equal(await createButton.isEnabled(), false, "The accepted creation must block duplicate submission");
    assert.equal(await closeButton.isEnabled(), false, "The pending result must stay attached to its form");
    const requestCount = await browser.execute(() => window.__uiAudit.requests.filter(
      (item) => item.command === "create_agent_thread" && item.body.kind === "subagent",
    ).length);
    assert.equal(requestCount, 1);
    await press(await scope.$('button[aria-label="返回上一页"]'));
    const navigationGuard = await dialog("未保存修改");
    assert.match(await navigationGuard.getText(), /操作仍在进行/);
    assert.equal(await navigationGuard.$("button=操作仍在进行").isEnabled(), false);
    assert.equal(new URL(await browser.getUrl()).pathname, "/agents");
    await click("取消", navigationGuard);
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const index = audit.pending.findIndex((item) => item.command === "create_agent_thread" && item.body.kind === "subagent");
      const [request] = audit.pending.splice(index, 1);
      audit.deferred = audit.deferred.filter((command) => command !== "create_agent_thread");
      request.resolve(Response.json({
        code: "unavailable", message: "模拟子 Agent 创建失败", retryable: true, trace_id: null, details: {},
      }, { status: 503 }));
    });
    await browser.waitUntil(async () => (await $("body").getText()).includes("模拟子 Agent 创建失败"), { timeout });
    assert.equal(await createButton.isEnabled(), true, "A failed creation must restore the explicit retry action");
    await browser.waitUntil(async () => !await $('[role="dialog"]').isExisting());
    await click("关闭", form);
    const discard = await dialog("未保存修改");
    await click("放弃草稿", discard);
    await browser.waitUntil(async () => !(await form.isDisplayed().catch(() => false)));
    current.visited.push(1440);
    current.evidence = { inFlightRequests: requestCount, duplicateSubmissionDisabled: true, failureRestoredForm: true };
  });

  it("UX-1781 keeps a created sub-agent visible when the thread-list refresh fails", async function () {
    current = { id: "UX-1781-subagent-result-refresh-error", task: "子 Agent 创建成功但列表读取失败时保留创建回包", visited: [], plannedWidths: [1440] };
    const scope = await agent();
    await click("新建子 Agent", scope);
    const form = await scope.$(".agent-monitor-create-form");
    await form.$('[aria-label="线程名称（可选）"]').setValue("刷新失败后保留的子 Agent");
    await form.$('[aria-label="子 Agent 能力域"]').setValue("director");
    const created = {
      ...mainSeed.thread,
      id: "thread-created-ux-1781",
      title: "刷新失败后保留的子 Agent",
      kind: "subagent",
      capability: "director",
      scope: { ...mainSeed.thread.scope, allowed_capabilities: ["director"] },
    };
    const createdSnapshot = { ...mainSeed, thread: created, approvals: [], delegations: [], events: [], executions: [], inbox: [], memories: [], runs: [], turns: [], messages: [] };
    await browser.execute(({ createdThread, snapshot }) => {
      const audit = window.__uiAudit;
      audit.fixtures.create_agent_thread = createdThread;
      audit.fixtures.agentSnapshots.push(snapshot);
      audit.deferred.push("create_agent_thread");
    }, { createdThread: created, snapshot: createdSnapshot });
    await click("创建", form);
    await browser.waitUntil(() => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "create_agent_thread"),
    ), { timeout });
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.failNext.push({
        command: "agent-threads", status: 503, message: "线程列表刷新失败",
      });
      const index = audit.pending.findIndex((item) => item.command === "create_agent_thread");
      const [pending] = audit.pending.splice(index, 1);
      audit.deferred = audit.deferred.filter((command) => command !== "create_agent_thread");
      pending.resolve(Response.json(audit.fixtures.create_agent_thread));
    });
    const createdRow = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)="刷新失败后保留的子 Agent"]]');
    await createdRow.waitForDisplayed({ timeout });
    await browser.waitUntil(async () => (await scope.getText()).includes("线程列表刷新失败"), { timeout });
    await browser.waitUntil(async () => (await scope.$(".agent-monitor-session-header h2").getText()) === created.title, { timeout });
    current.visited.push(1440);
    current.evidence = { createdThreadId: created.id, rowRetainedAfterListError: true, selectedCreatedThread: true };
  });

  const homePages = [
    { id: "H01-recent", task: "首页最近项目与同源派生项目", open: home, check: (scope) => expectProjects(scope, fixtures.projectRecords), last: async (scope) => (await scope.$$(".a4n-project-row")).at(-1) },
    { id: "H02-all", task: "切换到按标题分组的全部项目", open: async () => { const scope = await home(); await click("全部项目", scope); return scope; }, check: async (scope) => {
      const evidence = await expectProjects(scope, fixtures.projectRecords);
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "list_projects").body);
      assert.equal(request.pageSize, 100);
      return { ...evidence, pageSize: request.pageSize };
    }, last: async (scope) => (await scope.$$(".a4n-project-row")).at(-1) },
    ...[["text", "小说项目"], ["comic", "漫画项目"], ["video", "视频项目"], ["galgame", "GALGame 项目"]].map(([kind, label]) => ({
      id: "H03-type-" + kind, task: "按" + label + "筛选项目", open: async () => { const scope = await home(); await choose("项目类型", label, scope); return scope; },
      check: async (scope) => {
        const evidence = await expectProjects(scope, fixtures.projectRecords.filter((project) => project.project_kind === kind));
        const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "list_projects").body);
        assert.equal(request.projectKind, kind); return evidence;
      },
    })),
    ...[["owned", "我创建的"], ["collaborating", "我协作的"]].map(([access, label]) => ({
      id: "H04-access-" + access, task: "按" + label + "筛选访问范围", open: async () => { const scope = await home(); await choose("访问权限", label, scope); return scope; },
      check: async (scope) => {
        const evidence = await expectProjects(scope, fixtures.projectRecords.filter((project) => (project.my_role === "owner") === (access === "owned")));
        const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "list_projects").body);
        assert.equal(request.accessScope, access); return evidence;
      },
    })),
    { id: "H05-invites", task: "阅读项目邀请并打开拒绝菜单", open: async () => {
      const scope = await home();
      const invite = await scope.$('.//div[contains(@class,"a4n-flat-surface") and .//div[normalize-space(.)=' + literal(homeAgentFixtures.list_my_project_invites[0].project_title) + ']]');
      await press(await invite.$('button[aria-label="更多操作"]'));
      await $('[role="menuitem"]').waitForDisplayed({ timeout });
      return scope;
    }, text: "待处理项目邀请", check: async () => {
      assert.ok(await (await button("接受")).isEnabled());
      assert.ok(await $('//*[@role="menuitem" and contains(normalize-space(.),"拒绝邀请")]').isDisplayed());
      return { pendingInvites: homeAgentFixtures.list_my_project_invites.length };
    } },
    { id: "H06-create-options", task: "检查 Video 与 Canvas 独立创建入口", open: async () => { await home(); await click("创建项目"); return dialog("新建项目"); }, check: async (scope) => {
      const labels = ["AI 辅助创作", "自由创作", "视频项目", "独立画布项目"];
      for (const label of labels) {
        const option = await scope.$('.//button[.//span[normalize-space(.)=' + literal(label) + ']]');
        assert.ok((await reach(option)).reached); assert.ok(await option.isEnabled());
      }
      const videoOption = await scope.$('.//button[.//span[normalize-space(.)=' + literal("视频项目") + ']]');
      await press(videoOption);
      const sourceDialog = await dialog("选择视频来源项目");
      const sourceProject = fixtures.projectRecords.find((project) => project.project_kind === "text");
      assert.ok(sourceProject, "The fixture must expose a text project as a video source");
      const sourceSelect = await sourceDialog.$('button[role="combobox"][aria-label="文本项目来源"]');
      await sourceSelect.waitForDisplayed({ timeout });
      await browser.waitUntil(async () => (await sourceSelect.getText()).trim() === sourceProject.title, { timeout });
      const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "list_projects").body);
      assert.equal(request.projectKind, "text");
      return { creationOptions: labels, videoSourceProject: sourceProject.title, sourceKind: request.projectKind };
    } },
    { id: "H07-project-import", task: "检查项目包选择与导入确认入口", open: async () => { await home(); await click("导入项目"); return dialog("导入项目"); }, check: async (scope) => {
      assert.ok(await (await button("选择文件", scope)).isEnabled());
      assert.equal(await (await button("确认导入", scope)).isEnabled(), false);
      return { packageSelectionRequired: true };
    } },
    { id: "H08-text-import", task: "从首页项目操作打开有内容的正文导入窗口", open: async () => {
      const scope = await home();
      const root = fixtures.projectRecords.find((project) => project.id === 1);
      const row = await scope.$('.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=' + literal(root.title) + ']]');
      await press(await row.$('button[aria-label="更多操作"]')); await menu("导入文本到此项目");
      return dialog("导入项目文本");
    }, text: "12 条记录", last: async (scope) => scope.$('button[aria-label="查看 12-旧书店与车站调查补充材料.md 的审核详情"]') },
    { id: "H09-delete-read-model", task: "删除项目后，在列表刷新响应前不再暴露已删除项目", open: async () => {
      const scope = await home();
      const root = fixtures.projectRecords.find((project) => project.id === 1);
      const row = await scope.$('.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=' + literal(root.title) + ']]');
      await press(await row.$('button[aria-label="更多操作"]'));
      await menu("删除项目");
      const confirmation = await dialog("删除项目");
      await browser.execute(() => {
        window.__uiAudit.fixtures.delete_project = null;
        window.__uiAudit.deferred.push("list_projects");
      });
      await click("永久删除", confirmation);
      await browser.waitUntil(async () => browser.execute(() =>
        window.__uiAudit.pending.some((item) => item.command === "list_projects"),
      ), { timeout });
      await browser.waitUntil(async () => browser.execute((title) =>
        ![...document.querySelectorAll(".a4n-project-row h3")].some((heading) => heading.textContent?.trim() === title),
      root.title), { timeout, timeoutMsg: "A deleted project must leave the visible list before its refresh completes" });
      await browser.execute((projectId) => {
        const audit = window.__uiAudit;
        const pending = audit.pending.findLast((item) => item.command === "list_projects");
        const fixture = audit.fixtures.list_projects;
        const items = fixture.items.filter((project) => project.id !== projectId);
        audit.pending = audit.pending.filter((item) => item !== pending);
        audit.deferred = audit.deferred.filter((command) => command !== "list_projects");
        pending?.resolve(Response.json({ ...fixture, items, total: items.length }));
      }, root.id);
      await browser.waitUntil(async () => browser.execute((title) =>
        document.body.innerText.includes(`已删除：${title}`),
      root.title), { timeout });
      return scope;
    }, check: async (scope) => {
      const root = fixtures.projectRecords.find((project) => project.id === 1);
      assert.equal(await scope.$(`.//h3[normalize-space(.)=${literal(root.title)}]`).isExisting(), false);
      return { deletedProject: root.id, absentBeforeRefreshResponse: true };
    } },
  ];
  for (const spec of homePages) it(spec.id + " follows the existing home entry", async () => matrix(spec));

  it("UX-2238 preserves the collaboration filter when returning from a project", async function () {
    current = { id: "UX-2238-home-access-filter-return", task: "筛选协作项目、打开一项并返回首页时保留原访问范围", visited: [], plannedWidths: [1440] };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    await choose("访问权限", "我协作的", scope);
    const collaborating = fixtures.projectRecords.filter((project) => project.my_role !== "owner");
    await expectProjects(scope, collaborating);
    const selectedProject = collaborating[0];
    const row = await scope.$('.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=' + literal(selectedProject.title) + ']]');
    await press(await row.$('button[aria-label]'));
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname !== "/", { timeout });
    await browser.back();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/", { timeout });
    const returnedHome = await $(".a4n-home-content");
    await returnedHome.waitForDisplayed({ timeout });
    await browser.waitUntil(async () => (await returnedHome.$('button[role="combobox"][aria-label="访问权限"]').getText()).includes("我协作的"), { timeout });
    await expectProjects(returnedHome, collaborating);
    const request = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "list_projects").body);
    assert.equal(request.accessScope, "collaborating");
    current.visited.push(1440);
    current.evidence = { selectedProject: selectedProject.id, scopePreserved: true, returnedRequestScope: request.accessScope };
  });

  it("UX-1926 ignores a project-list read superseded by a newer mutation", async function () {
    current = { id: "UX-1926-list-refresh-superseded", task: "较新的项目列表读取成功后不把旧响应报告为刷新失败" };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    const projectToDelete = fixtures.projectRecords.find((project) => project.id === 2);
    const row = await scope.$('.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=' + literal(projectToDelete.title) + ']]');
    await press(await row.$('button[aria-label="更多操作"]'));
    await menu("删除项目");
    const confirmation = await dialog("删除项目");
    await browser.execute((projectId) => {
      const audit = window.__uiAudit;
      audit.fixtures.delete_project = null;
      audit.fixtures.accept_project_invite = null;
      audit.fixtures.list_projects = {
        ...audit.fixtures.list_projects,
        items: audit.fixtures.list_projects.items.filter((project) => project.id !== projectId),
      };
      audit.fixtures.list_my_project_invites = audit.fixtures.list_my_project_invites.filter((invite) => invite.id !== 8101);
      audit.deferred.push("list_projects");
    }, projectToDelete.id);
    await click("永久删除", confirmation);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.filter((item) => item.command === "list_projects").length === 1,
    ), { timeout, timeoutMsg: "The deletion's project-list refresh must remain pending" });

    await click("接受", scope);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.filter((item) => item.command === "list_projects").length === 2,
    ), { timeout, timeoutMsg: "Accepting an invite must start a newer project-list refresh" });
    await browser.execute((invite) => {
      const audit = window.__uiAudit;
      const source = audit.fixtures.projectRecords.find((project) => project.id === 1);
      audit.fixtures.list_projects.items.push({
        ...source,
        id: 99,
        title: invite.project_title,
        collaboration_root_project_id: 99,
        my_role: "editor",
        owner_username: invite.inviter_username,
        pending_invite_count: 0,
      });
    }, homeAgentFixtures.list_my_project_invites[0]);

    const resolvePendingList = async (fromEnd, responseStatus = 200) => browser.execute((offset, status) => {
      const audit = window.__uiAudit;
      const pendingItems = audit.pending.filter((item) => item.command === "list_projects");
      const pending = pendingItems.at(offset);
      if (!pending) throw new Error("Missing pending project-list response");
      const fixture = audit.fixtures.list_projects;
      if (status !== 200) {
        audit.pending = audit.pending.filter((item) => item !== pending);
        pending.resolve(Response.json({
          code: "unavailable",
          message: "较早的项目列表读取失败",
          retryable: true,
          trace_id: null,
          details: {},
        }, { status }));
        return;
      }
      const page = pending.body.page ?? 1;
      const pageSize = pending.body.pageSize ?? 20;
      const access = pending.body.accessScope ?? "all";
      const items = fixture.items.filter((project) => access === "all"
        || (access === "owned" ? project.my_role === "owner" : project.my_role !== "owner"));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "list_projects");
      pending.resolve(Response.json({
        ...fixture,
        items: items.slice((page - 1) * pageSize, page * pageSize),
        page,
        page_size: pageSize,
        total: items.length,
      }));
    }, fromEnd, responseStatus);
    await resolvePendingList(-1);
    await browser.waitUntil(async () => browser.execute(() =>
      document.body.innerText.includes("已接受项目邀请"),
    ), { timeout });
    await browser.waitUntil(async () => browser.execute((title) =>
      ![...document.querySelectorAll(".a4n-project-row h3")].some((heading) => heading.textContent?.trim() === title),
    projectToDelete.title), { timeout });
    await resolvePendingList(0, 503);
    await browser.waitUntil(async () => browser.execute((title) =>
      document.body.innerText.includes(`已删除：${title}`),
    projectToDelete.title), { timeout });
    assert.equal(await browser.execute(() => document.body.innerText.includes("项目列表刷新失败")), false);
    assert.equal(await scope.$(`.//h3[normalize-space(.)=${literal(projectToDelete.title)}]`).isExisting(), false);
    await shot(scope, 1440, "superseded-project-list-refresh", { deletedProject: projectToDelete.id, latestReadApplied: true, staleReadReportedAsSuccess: true });
  });

  it("UX-1927 reloads an invitation after another session has already accepted it", async function () {
    current = { id: "UX-1927-stale-invitation", task: "外部已接受的邀请不再留在首页可操作列表" };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    const invite = homeAgentFixtures.list_my_project_invites[0];
    const row = () => scope.$('.//div[contains(@class,"a4n-flat-surface") and .//div[normalize-space(.)=' + literal(invite.project_title) + ']]');
    const listReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_my_project_invites").length,
    );
    await browser.execute((inviteId) => {
      const audit = window.__uiAudit;
      audit.fixtures.list_my_project_invites = audit.fixtures.list_my_project_invites.filter((item) => item.id !== inviteId);
      audit.failNext.push({ command: "accept_project_invite", status: 409, message: "邀请已不可接受" });
    }, invite.id);
    await click("接受", await row());
    await waitText($("body"), "邀请已不可接受");
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_my_project_invites").length === count + 1,
    listReads), { timeout });
    await browser.waitUntil(async () => !await (await row()).isExisting(), { timeout });
    await shot(scope, 1440, "stale-invitation-removed", { inviteId: invite.id, directoryReloaded: true, staleRowRemoved: true });
  });

  it("UX-2268 refreshes accessible projects after an invitation is accepted elsewhere", async function () {
    current = { id: "UX-2268-invitation-project-list-sync", task: "另一窗口接受邀请后，本地接受失败也应把新协作项目加入列表" };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    const invite = homeAgentFixtures.list_my_project_invites[0];
    const row = () => scope.$('.//div[contains(@class,"a4n-flat-surface") and .//div[normalize-space(.)=' + literal(invite.project_title) + ']]');
    const listReads = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "list_projects").length,
    );
    await browser.execute(({ inviteId, invite }) => {
      const audit = window.__uiAudit;
      const source = audit.fixtures.projectRecords.find((project) => project.id === 1);
      const acceptedProject = {
        ...source,
        id: 99,
        title: invite.project_title,
        collaboration_root_project_id: 99,
        my_role: "editor",
        owner_username: invite.inviter_username,
        pending_invite_count: 0,
      };
      audit.fixtures.list_my_project_invites = audit.fixtures.list_my_project_invites.filter((item) => item.id !== inviteId);
      audit.fixtures.projectRecords.push(acceptedProject);
      audit.fixtures.list_projects = {
        ...audit.fixtures.list_projects,
        items: [...audit.fixtures.list_projects.items, acceptedProject],
        total: audit.fixtures.list_projects.total + 1,
      };
      audit.failNext.push({ command: "accept_project_invite", status: 409, message: "邀请已由其他窗口接受" });
    }, { inviteId: invite.id, invite });
    await click("接受", await row());
    await browser.waitUntil(async () => (await $("body").getText()).includes("邀请已由其他窗口接受"), { timeout });
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "list_projects").length > count,
    listReads), { timeout });
    await browser.waitUntil(async () => (await projectTitles(scope)).includes(invite.project_title), { timeout });
    await browser.waitUntil(async () => !await (await row()).isExisting(), { timeout });
    await shot(scope, 1440, "accepted-invitation-project-visible", {
      failedLocalAccept: true,
      externalProjectNowAccessible: true,
    });
  });

  it("UX-1928 hides stale invitation actions when the follow-up directory read fails", async function () {
    current = { id: "UX-1928-invitation-read-failure", task: "邀请操作失败后的目录读取失败不保留旧的接受按钮" };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    const invite = homeAgentFixtures.list_my_project_invites[0];
    const row = () => scope.$('.//div[contains(@class,"a4n-flat-surface") and .//div[normalize-space(.)=' + literal(invite.project_title) + ']]');
    await browser.execute(() => {
      const audit = window.__uiAudit;
      audit.failNext.push(
        { command: "accept_project_invite", status: 409, message: "邀请已不可接受" },
        { command: "list_my_project_invites", status: 503, message: "邀请目录读取失败" },
      );
    });
    await click("接受", await row());
    await waitText($("body"), "邀请目录读取失败");
    await browser.waitUntil(async () => !await (await row()).isExisting(), { timeout });
    assert.ok(await (await button("重试", scope)).isDisplayed());
    await shot(scope, 1440, "stale-invitation-directory-error", { inviteId: invite.id, staleActionsHidden: true, retryVisible: true });
  });

  it("UX-1929 reports invite-directory retry as loading instead of retaining the old error", async function () {
    current = { id: "UX-1929-invitation-retry-state", task: "重试邀请目录时清除旧错误并显示当前读取状态" };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    const invite = homeAgentFixtures.list_my_project_invites[0];
    const row = () => scope.$('.//div[contains(@class,"a4n-flat-surface") and .//div[normalize-space(.)=' + literal(invite.project_title) + ']]');
    await browser.execute(() => {
      window.__uiAudit.failNext.push(
        { command: "accept_project_invite", status: 409, message: "邀请已不可接受" },
        { command: "list_my_project_invites", status: 503, message: "邀请目录读取失败" },
      );
    });
    await click("接受", await row());
    await waitText($("body"), "邀请目录读取失败");
    await browser.execute(() => window.__uiAudit.deferred.push("list_my_project_invites"));
    await click("重试", scope);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "list_my_project_invites"),
    ), { timeout });
    assert.equal(await browser.execute(() => document.body.innerText.includes("邀请目录读取失败")), false);
    assert.ok(await scope.$('[role="status"]').isDisplayed());
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const pending = audit.pending.findLast((item) => item.command === "list_my_project_invites");
      if (!pending) throw new Error("Expected the deferred invitation-directory response");
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "list_my_project_invites");
      pending.resolve(Response.json(audit.fixtures.list_my_project_invites));
    });
    await browser.waitUntil(async () => await (await row()).isExisting(), { timeout });
    await shot(scope, 1440, "invitation-retry-loading-state", { oldErrorCleared: true, loadingFeedbackVisible: true, inviteRestored: true });
  });

  it("UX-1930 indexes text projects restored from a unified project bundle", async function () {
    current = { id: "UX-1930-bundle-project-rag", task: "恢复统一项目包后同步文本项目的 RAG 索引" };
    await browser.setWindowSize(1440, 1000);
    const scope = await home();
    const source = fixtures.projectRecords.find((project) => project.project_kind === "text");
    const imported = { ...source, id: 990, title: "恢复后的文本项目", collaboration_root_project_id: 990 };
    await browser.execute((project) => {
      const audit = window.__uiAudit;
      audit.fixtures.import_project_bundle_upload = {
        project_summaries: [project],
        conversations_imported: 0,
        messages_imported: 0,
        workflow_runs_imported: 0,
        workflow_runs_marked_interrupted: 0,
        generation_runs_imported: 0,
        missing_provider_slots: [],
        warnings: ["恢复后的文本项目缺少模型配置"],
      };
      audit.fixtures.list_projects.items.push(project);
      audit.fixtures.list_projects.total = audit.fixtures.list_projects.items.length;
      audit.fixtures.rag_ingest_all = { added_count: 1, updated_count: 0, skipped_count: 0, deleted_count: 0 };
      audit.fixtures.get_workflow_progress_events = { events: [], stream_text: null };
      const prototype = HTMLInputElement.prototype;
      window.__bundleFilePickerRestore = { click: prototype.click, showPicker: prototype.showPicker };
      prototype.click = () => {};
      if (prototype.showPicker) prototype.showPicker = () => {};
    }, imported);
    await click("导入项目", scope);
    const importDialog = await dialog("导入项目");
    await click("选择文件", importDialog);
    await browser.execute(() => {
      const input = document.querySelector('input[type="file"]');
      if (!input) throw new Error("Project import did not create its file input");
      const transfer = new DataTransfer();
      transfer.items.add(new File(["fixture package"], "restored-project.a4nproj", { type: "application/zip" }));
      input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true }));
      const restore = window.__bundleFilePickerRestore;
      HTMLInputElement.prototype.click = restore.click;
      if (restore.showPicker) HTMLInputElement.prototype.showPicker = restore.showPicker;
      delete window.__bundleFilePickerRestore;
    });
    await click("确认导入", importDialog);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.requests.some((item) => item.command === "rag_ingest_all"),
    ), { timeout });
    const ingest = await browser.execute(() => window.__uiAudit.requests.findLast((item) => item.command === "rag_ingest_all"));
    assert.equal(ingest.body.projectId, imported.id);
    await waitText($("body"), "已导入统一项目包");
    await waitText($("body"), "恢复后的文本项目缺少模型配置");
    assert.ok(await scope.$(`.//h3[normalize-space(.)=${literal(imported.title)}]`).isDisplayed());
    await shot(scope, 1440, "bundle-project-indexed", { importedProjectId: imported.id, ragProjectId: ingest.body.projectId, warningVisible: true });
  });

  const agentPages = [
    { id: "A01-main", task: "读取主线程历史并检查消息输入", open: () => agent(), text: "当前正文没有写入", check: async (scope) => {
      assert.equal((await scope.$$('[data-agent-message-role]')).length, mainSeed.messages.length);
      await scope.$('[aria-label="Agent 消息"]').setValue("保留第一章原文，先继续讨论核验顺序。");
      assert.ok(await (await button("发送消息", scope)).isEnabled());
      return { threadId: mainSeed.thread.id, messages: mainSeed.messages.length, sendEnabled: true };
    }, last: async (scope) => scope.$(".agent-message-list > :last-child") },
    { id: "A02-child", task: "切换到预算暂停的子线程历史", open: () => agent(budgetSeed.thread.id), text: "当前预算已用完", check: async (scope) => ({ selectedThread: await scope.$(".agent-monitor-session-header h2").getText() }), last: async (scope) => scope.$(".agent-message-list > :last-child") },
    { id: "A03-activity", task: "打开主线程轨迹和运行控制，返回时保留草稿", open: async () => {
      const scope = await agent();
      await scope.$('[aria-label="Agent 消息"]').setValue(activityDraft);
      await press(await scope.$(".agent-monitor-activity-toggle"));
      await scope.$(".agent-monitor-activity-pane").waitForDisplayed({ timeout });
      return scope;
    }, text: "等待中", check: async (scope, width) => {
      assert.ok(await (await button("暂停", scope)).isEnabled());
      const layout = await scope.execute((root) => {
        const workspace = root.querySelector(".agent-monitor-workspace").getBoundingClientRect();
        const pane = root.querySelector(".agent-monitor-activity-pane").getBoundingClientRect();
        const conversation = root.querySelector(".agent-monitor-conversation");
        return {
          workspaceWidth: workspace.width, activityWidth: pane.width,
          conversationDisplay: getComputedStyle(conversation).display,
        };
      });
      if (width <= 1099) {
        assert.equal(layout.conversationDisplay, "none", "Covered chat must leave the visual and accessibility trees");
        assert.equal(await scope.$('[aria-label="Agent 消息"]').isDisplayed(), false);
        assert.ok(Math.abs(layout.activityWidth - layout.workspaceWidth) <= 1, "The activity pane must fill the available workspace: " + JSON.stringify(layout));
      } else {
        assert.ok(await scope.$('[aria-label="Agent 消息"]').isDisplayed());
      }
      return { pauseEnabled: true, layout, draftBeforeActivity: activityDraft };
    }, after: async (scope) => {
      await press(await scope.$(".agent-monitor-activity-toggle"));
      const input = await scope.$('[aria-label="Agent 消息"]');
      await input.waitForDisplayed({ timeout });
      assert.equal(await input.getValue(), activityDraft, "Returning from activity must preserve the thread draft");
      await press(input);
      await shot(scope, current.width, "return-to-chat", { restoredDraft: await input.getValue(), editorFocused: await input.isFocused() });
    } },
    { id: "A04-approvals", task: "读取审批原因并发现批准与拒绝操作", open: () => activitySection("待审批操作"), text: mainSeed.approvals[0].reason, check: async (scope) => {
      assert.ok(await (await button("批准", scope)).isEnabled()); assert.ok(await (await button("拒绝", scope)).isEnabled());
      return { approvalId: mainSeed.approvals[0].id, command: mainSeed.approvals[0].command };
    }, last: async (scope) => button("拒绝", scope) },
    { id: "A04a-approval-exclusion", task: "处理一项审批时锁定同线程其他待审批项", open: async () => {
      for (let index = mainSeed.approvals.length - 1; index >= 0; index--) {
        if (mainSeed.approvals[index].id === "home-approval-second") mainSeed.approvals.splice(index, 1);
      }
      mainSeed.approvals.push({
        ...structuredClone(mainSeed.approvals[0]),
        id: "home-approval-second",
        execution_id: "home-execution-second-approval",
        command: "update_project_material",
        reason: "用于验证同线程只能同时处理一项审批。",
      });
      await installUiFixtures(browser, fixtures);
      const scope = await agent(mainSeed.thread.id, true);
      const firstApproval = mainSeed.approvals[0];
      const firstCard = await scope.$(`[data-approval-id="${firstApproval.id}"]`);
      await browser.execute((approvalId) => window.__uiAudit.deferred.push(approvalId), firstApproval.id);
      await click("批准", firstCard);
      try {
        await browser.waitUntil(async () => browser.execute((approvalId) =>
          window.__uiAudit.pending.some((item) => item.command === approvalId), firstApproval.id), { timeout });
      } catch (error) {
        const state = await browser.execute(() => ({
          requests: window.__uiAudit.requests.slice(-5),
          deferred: window.__uiAudit.deferred,
          pending: window.__uiAudit.pending.map((item) => item.command),
        }));
        throw new Error(`${error.message}; UI fixture state=${JSON.stringify(state)}`);
      }
      const secondCard = await scope.$('[data-approval-id="home-approval-second"]');
      assert.equal(await (await button("批准", secondCard)).isEnabled(), false);
      assert.equal(await (await button("拒绝", secondCard)).isEnabled(), false);
      await browser.execute((approvalId) => {
        const pending = window.__uiAudit.pending.find((item) => item.command === approvalId);
        const approval = window.__uiAudit.fixtures.agentSnapshots[0].approvals.find((item) => item.id === approvalId);
        Object.assign(approval, { status: "approved", decision: "approve" });
        pending.resolve(Response.json(approval));
      }, firstApproval.id);
      try {
        await browser.waitUntil(async () => {
          const currentCard = await $('[data-approval-id="home-approval-second"]');
          return (await button("批准", currentCard)).isEnabled();
        }, { timeout });
      } catch (error) {
        const state = await browser.execute(() => ({
          approvals: window.__uiAudit.fixtures.agentSnapshots[0].approvals.map(({ id, status }) => ({ id, status })),
          requests: window.__uiAudit.requests.slice(-5),
          pending: window.__uiAudit.pending.map((item) => item.command),
          buttons: [...document.querySelectorAll('[data-approval-id="home-approval-second"] button')]
            .map((item) => ({ disabled: item.disabled, text: item.innerText })),
        }));
        throw new Error(`${error.message}; UI fixture state=${JSON.stringify(state)}`);
      }
      assert.equal(await browser.execute((approvalId) =>
        window.__uiAudit.requests.filter((item) => item.command === approvalId).length, firstApproval.id), 1);
      return scope;
    }, text: "用于验证同线程只能同时处理一项审批。", check: async (scope) => {
      assert.equal(await (await button("批准", await scope.$('[data-approval-id="home-approval-second"]'))).isEnabled(), true);
      return { approvalRequests: 1, secondApprovalEnabledAfterCompletion: true };
    } },
    { id: "A05-executions", task: "查看读取、模型、委派与待审阅执行记录", open: () => activitySection("执行"), text: "list_chapters", check: async (scope) => {
      assert.equal((await scope.$$(":scope > div > div")).length, mainSeed.executions.length);
      return { executionRecords: mainSeed.executions.length };
    }, last: async (scope) => (await scope.$$(":scope > div > div")).at(-1) },
    { id: "A06-events", task: "检查持久事件序号与文字说明", open: () => activitySection("事件轨迹"), text: "等待用户确认启动正文优化子任务", check: async (scope) => {
      const text = await scope.getText(); assert.ok(text.includes("#8") && text.includes("#1"));
      return { eventRecords: mainSeed.events.length };
    }, last: async (scope) => (await scope.$$(":scope > div > div")).at(-1) },
    { id: "A07-thread-details", task: "展开当前线程的诊断信息", open: async () => {
      const scope = await agent(budgetSeed.thread.id, true);
      const details = await scope.$(".agent-monitor-activity-pane details");
      await press(await details.$("summary")); return details;
    }, text: "project:1", last: async (scope) => (await scope.$$("dd")).at(-1) },
    { id: "A08-create-input", task: "填写实际存在的子线程名称和能力域", open: async () => {
      const scope = await agent(); await click("新建子 Agent", scope);
      const form = await scope.$(".agent-monitor-create-form");
      await form.$('[aria-label="线程名称（可选）"]').setValue("场景与动作连续性复审");
      await form.$('[aria-label="子 Agent 能力域"]').setValue("director"); return form;
    }, check: async (scope) => {
      assert.ok(await (await button("创建", scope)).isEnabled());
      return { title: await scope.$('[aria-label="线程名称（可选）"]').getValue(), capability: await scope.$('[aria-label="子 Agent 能力域"]').getValue() };
    } },
    { id: "A09-clear-confirm", task: "阅读清空上下文确认并取消，保留原线程", open: async () => {
      const scope = await agent(); await click("清空 Agent 上下文", scope); return dialog("清空 Agent 上下文");
    }, text: "旧消息会保留在历史记录中", after: async (scope) => {
      await click("取消", scope); await scope.waitForDisplayed({ reverse: true, timeout });
      assert.equal(await $(".agent-monitor-session-header h2").getText(), mainSeed.thread.title);
    } },
    { id: "A10-budget", task: "读取预算归属、已用额度与新的总上限", open: budgetDialog, text: "已确认 " + new Intl.NumberFormat("zh-CN").format(budgetSeed.runs[0].tokens_used) + " token", check: async (scope) => {
      const values = await Promise.all([...await scope.$$("input")].map((input) => input.getValue()));
      assert.deepEqual(values, ["24", "16", "8", "12000"]);
      assert.ok(await (await button("继续已有任务", scope)).isEnabled());
      return { runId: budgetSeed.runs[0].id, suggestedLimits: values };
    }, last: async (scope) => button("继续已有任务", scope) },
    { id: "A11-budget-invalid", task: "检查不能降低既有预算上限的反馈", open: async () => {
      const scope = await budgetDialog(); await scope.$("input").setValue("0"); return scope;
    }, text: "不能降低已有上限", check: async (scope) => {
      assert.equal(await (await button("继续已有任务", scope)).isEnabled(), false);
      assert.ok(await scope.$('[role="alert"]').isDisplayed()); return { invalidBudgetBlocked: true };
    } },
    { id: "A12-delegation-entry", task: "从委派记录打开对应子线程", open: async () => {
      const scope = await agent(mainSeed.thread.id, true);
      await press(await scope.$('.//button[contains(normalize-space(.),"打开子线程")]'));
      await browser.waitUntil(async () => (await scope.$(".agent-monitor-session-header h2").getText()) === budgetSeed.thread.title, { timeout });
      return scope;
    }, text: "当前预算已用完" },
  ];
  for (const spec of agentPages) it(spec.id + " follows the existing Agent entry", async () => matrix(spec));

  it("UX-1746 keeps a failed snapshot notice scoped to its thread", async () => {
    current = { id: "UX-1746", task: "Keep a prior thread's read failure out of the newly selected thread" };
    const scope = await agent(mainSeed.thread.id, true);
    const child = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)=' + literal(budgetSeed.thread.title) + ']]');
    await browser.execute((threadId) => window.__uiAudit.failNext.push({
      command: threadId, status: 503, message: "子线程快照读取失败",
    }), budgetSeed.thread.id);
    await press(child);
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.requests.some((request) => request.command === threadId), budgetSeed.thread.id), { timeout });
    await browser.waitUntil(async () => (await scope.getText()).includes("子线程快照读取失败"), { timeout });

    await browser.execute((threadId) => window.__uiAudit.deferred.push(threadId), mainSeed.thread.id);
    const main = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)=' + literal(mainSeed.thread.title) + ']]');
    await press(main);
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.pending.some((item) => item.command === threadId), mainSeed.thread.id), { timeout });
    assert.equal((await scope.getText()).includes("子线程快照读取失败"), false,
      "A read failure from the previous thread must not be presented as the selected thread's error");
    await shot(scope, 1440, "new-thread-read-pending", { previousThreadErrorHidden: true });

    await browser.execute((threadId) => {
      const audit = window.__uiAudit;
      const pending = audit.pending.findLast((item) => item.command === threadId);
      if (!pending) throw new Error("Expected the deferred main-thread snapshot");
      const snapshot = audit.fixtures.agentSnapshots.find((item) => item.thread.id === threadId);
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== threadId);
      pending.resolve(Response.json(snapshot));
    }, mainSeed.thread.id);
    await browser.waitUntil(async () =>
      (await scope.$$('[data-agent-message-role]')).length === mainSeed.messages.length, { timeout });
    assert.equal((await scope.getText()).includes("子线程快照读取失败"), false);
    await shot(scope, 1440, "new-thread-read-complete", { selectedThread: mainSeed.thread.id, priorThreadErrorHidden: true });
  });

  it("UX-1747 releases the manual refresh control when polling supersedes it", async () => {
    current = { id: "UX-1747", task: "Release the refresh control when a periodic read supersedes the manual read" };
    await browser.addInitScript(() => {
      const originalSetInterval = window.setInterval.bind(window);
      window.__agentMonitorRefreshTick = null;
      window.setInterval = (callback, delay, ...args) => {
        if (delay === 15000) {
          window.__agentMonitorRefreshTick = callback;
          return 2147483647;
        }
        return originalSetInterval(callback, delay, ...args);
      };
    });
    const scope = await agent(mainSeed.thread.id);
    const refresh = await button("刷新 Agent 状态", scope);
    const previousCount = await browser.execute(() =>
      window.__uiAudit.requests.filter((request) => request.command === "agent-threads").length,
    );
    await browser.execute(() => window.__uiAudit.deferred.push("agent-threads"));
    await press(refresh);
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "agent-threads"),
    ), { timeout });
    await browser.execute(() => {
      window.__uiAudit.deferred = window.__uiAudit.deferred.filter((command) => command !== "agent-threads");
    });

    await browser.waitUntil(async () => browser.execute(() =>
      typeof window.__agentMonitorRefreshTick === "function",
    ), { timeout });
    await browser.execute(() => window.__agentMonitorRefreshTick());
    await browser.waitUntil(async () => browser.execute((count) =>
      window.__uiAudit.requests.filter((request) => request.command === "agent-threads").length === count + 2,
    previousCount), { timeout });
    await browser.waitUntil(async () => refresh.isEnabled(), { timeout,
      timeoutMsg: "The superseding periodic read should release the completed manual refresh control" });

    await browser.execute(() => {
      const audit = window.__uiAudit;
      const pending = audit.pending.findLast((item) => item.command === "agent-threads");
      if (!pending) throw new Error("Expected the deferred manual thread-list request");
      pending.resolve(Response.json(audit.fixtures.agentSnapshots.map((item) => item.thread)));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "agent-threads");
    });
    await shot(scope, 1440, "manual-refresh-released", { supersededByPolling: true, refreshEnabled: await refresh.isEnabled() });
  });

  it("UX-2136 and UX-2143 release accepted sends and keep each thread's pending messages", async () => {
    current = { id: "UX-2136", task: "Release send after command acceptance and retain its optimistic messages by thread" };
    await browser.setWindowSize(1440, height);
    const scope = await agent(mainSeed.thread.id);
    const composer = await scope.$('[aria-label="Agent 消息"]');
    await browser.execute((threadId) => window.__uiAudit.deferred.push(threadId, "agent-threads"), mainSeed.thread.id);

    const firstText = "先确认第一条已进入队列。";
    await composer.setValue(firstText);
    await press(await button("发送消息", scope));
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.agentMessages.length > 0), { timeout });
    const firstAccepted = await acceptPendingAgentSend();
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.pending.some((item) => item.command === threadId) &&
      window.__uiAudit.pending.some((item) => item.command === "agent-threads"),
    mainSeed.thread.id), { timeout });
    await browser.waitUntil(async () => composer.isEnabled(), { timeout,
      timeoutMsg: "The accepted send must release the composer while its snapshot reads are pending" });

    const secondText = "再补充一条待核验事项。";
    await composer.setValue(secondText);
    await press(await button("发送消息", scope));
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.agentMessages.length > 0), { timeout });
    const secondAccepted = await acceptPendingAgentSend();
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.pending.filter((item) => item.command === threadId).length >= 2,
    mainSeed.thread.id), { timeout });

    const child = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)=' + literal(budgetSeed.thread.title) + ']]');
    await press(child);
    await browser.waitUntil(async () =>
      (await scope.$(".agent-monitor-session-header h2").getText()) === budgetSeed.thread.title, { timeout });
    const main = await scope.$('.//button[contains(@class,"agent-monitor-thread-item") and .//span[normalize-space(.)=' + literal(mainSeed.thread.title) + ']]');
    await press(main);
    await browser.waitUntil(async () =>
      (await scope.$(".agent-monitor-session-header h2").getText()) === mainSeed.thread.title, { timeout });
    await browser.waitUntil(async () => {
      const text = await scope.$(".agent-message-list").getText();
      return text.includes(firstText) && text.includes(secondText);
    }, { timeout, timeoutMsg: "Switching threads must not discard accepted pending messages" });

    await resolvePendingAgentReads([mainSeed.thread.id]);
    await shot(scope, 1440, "accepted-pending-messages-restored", {
      acceptedMessageIds: [firstAccepted.message_id, secondAccepted.message_id],
      pendingMessagesVisible: 2,
      threadSwitchPreserved: true,
    });
  });

  it("UX-2137 releases run control after the control result, before snapshot reads", async () => {
    current = { id: "UX-2137", task: "Release pause control when its command response arrives" };
    await browser.setWindowSize(1440, height);
    const scope = await agent(mainSeed.thread.id, true);
    await browser.execute((threadId) => window.__uiAudit.deferred.push(threadId, "agent-threads"), mainSeed.thread.id);
    await press(await button("暂停", scope));
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.pending.some((item) => item.command === threadId) &&
      window.__uiAudit.pending.some((item) => item.command === "agent-threads"),
    mainSeed.thread.id), { timeout });
    const resume = await button("继续", scope);
    await browser.waitUntil(async () => resume.isEnabled(), { timeout,
      timeoutMsg: "A successful pause must release run controls while read models are pending" });
    await resolvePendingAgentReads([mainSeed.thread.id]);
    await shot(scope, 1440, "pause-result-before-read-models", { resumeEnabled: true, readsPendingAtResponse: true });
  });

  it("UX-2138 releases the next approval after the current approval result", async () => {
    current = { id: "UX-2138", task: "Release approval controls after the write result while snapshot refresh is pending" };
    mainSeed.approvals = mainSeed.approvals.map((approval) => ({ ...approval, status: "pending", decision: null }));
    if (!mainSeed.approvals.some((approval) => approval.id === "home-approval-second")) {
      mainSeed.approvals.push({
        ...structuredClone(mainSeed.approvals[0]),
        id: "home-approval-second",
        execution_id: "home-execution-second-approval",
        command: "update_project_material",
        reason: "验证前一项审批完成后下一项可操作。",
      });
    }
    await installUiFixtures(browser, fixtures);
    await browser.setWindowSize(1440, height);
    const scope = await agent(mainSeed.thread.id, true);
    const first = await scope.$(`[data-approval-id="${mainSeed.approvals[0].id}"]`);
    const second = await scope.$('[data-approval-id="home-approval-second"]');
    await browser.execute((threadId) => window.__uiAudit.deferred.push(threadId), mainSeed.thread.id);
    await press(await button("批准", first));
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.pending.some((item) => item.command === threadId),
    mainSeed.thread.id), { timeout });
    const secondApprove = await button("批准", second);
    await browser.waitUntil(async () => secondApprove.isEnabled(), { timeout,
      timeoutMsg: "The next approval must be usable once the first write has returned" });
    await resolvePendingAgentReads([mainSeed.thread.id]);
    await shot(scope, 1440, "approval-result-before-snapshot", { nextApprovalEnabled: true, snapshotPending: true });
  });

  it("UX-2142 closes the budget dialog after resume succeeds while reads continue", async () => {
    current = { id: "UX-2142", task: "Close the resumed budget dialog independently of background thread reads" };
    await browser.setWindowSize(1440, height);
    const modal = await budgetDialog();
    await browser.execute((threadId) => window.__uiAudit.deferred.push(threadId, "agent-threads"), budgetSeed.thread.id);
    await press(await button("继续已有任务", modal));
    await browser.waitUntil(async () => browser.execute((threadId) =>
      window.__uiAudit.pending.some((item) => item.command === threadId) &&
      window.__uiAudit.pending.some((item) => item.command === "agent-threads"),
    budgetSeed.thread.id), { timeout });
    await browser.waitUntil(async () => !await modal.isDisplayed(), { timeout,
      timeoutMsg: "Successful budget resume must close the dialog without waiting for thread reads" });
    await resolvePendingAgentReads([budgetSeed.thread.id]);
    await shot($("body"), 1440, "budget-resumed-before-reads", { dialogClosed: true, readsPendingAtResponse: true });
  });
});
