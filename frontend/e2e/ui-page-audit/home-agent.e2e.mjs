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
});
