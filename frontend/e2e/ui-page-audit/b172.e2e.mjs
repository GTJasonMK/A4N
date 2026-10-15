import assert from "node:assert/strict";
import { buildPageFixtures } from "./fixtures.mjs";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";

const timeout = 6000;
let auditWindow;
let fixtures;

const literal = (value) => JSON.stringify(value);

async function visibleButton(label, scope = null) {
  const selectors = [
    `button[aria-label=${literal(label)}]`,
    scope
      ? `.//button[normalize-space(.)=${literal(label)}]`
      : `//button[normalize-space(.)=${literal(label)}]`,
  ];
  for (const selector of selectors) {
    const candidates = await (scope ? scope.$$(selector) : $$(selector));
    for (const candidate of candidates) {
      if (await candidate.isDisplayed()) return candidate;
    }
  }
  throw new Error(`No visible button: ${label}`);
}

async function clickButton(label, scope = null) {
  const button = await visibleButton(label, scope);
  await button.waitForEnabled({ timeout });
  await button.click();
  return button;
}

async function activeDialog(title) {
  let result;
  await browser.waitUntil(async () => {
    for (const candidate of await $$('.a4n-modal-layer[data-modal-state="entering"] [role="dialog"]')) {
      const heading = await candidate.execute((node) => {
        const id = node.getAttribute("aria-labelledby");
        return id ? document.getElementById(id)?.textContent?.trim() : "";
      });
      if (heading === title && await candidate.isDisplayed()) {
        result = candidate;
        return true;
      }
    }
    return false;
  }, { timeout, timeoutMsg: `No active dialog: ${title}` });
  return result;
}

async function defer(command) {
  await browser.execute((name) => {
    window.__uiAudit.deferred.push(name);
  }, command);
}

async function pending(command) {
  await browser.waitUntil(async () => browser.execute((name) =>
    window.__uiAudit.pending.some((item) => item.command === name), command), {
    timeout,
    timeoutMsg: `No pending request for ${command}`,
  });
}

async function respond(command, body, status = 200) {
  await browser.execute(({ name, value, responseStatus }) => {
    const audit = window.__uiAudit;
    const request = audit.pending.findLast((item) => item.command === name);
    if (!request) throw new Error(`Missing pending request for ${name}`);
    audit.pending = audit.pending.filter((item) => item !== request);
    audit.deferred = audit.deferred.filter((item) => item !== name);
    request.resolve(Response.json(value, { status: responseStatus }));
  }, { name: command, value: body, responseStatus: status });
}

async function resetFixtures() {
  await browser.execute(() => {
    const audit = window.__uiAudit;
    audit.deferred = [];
    audit.pending = [];
    audit.failNext = [];
    audit.unexpected = [];
    audit.errors = [];
  });
}

async function home() {
  await browser.switchToWindow(auditWindow);
  await resetFixtures();
  await browser.url(`/?b172=${Date.now()}`);
  const scope = await $(".a4n-home-content");
  await scope.waitForDisplayed({ timeout });
  await browser.waitUntil(async () => (await scope.getText()).includes("项目工作台"), { timeout });
  return scope;
}

async function setFailure(command, message) {
  await browser.execute(({ name, failureMessage }) => {
    window.__uiAudit.failNext.push({ command: name, status: 503, message: failureMessage });
  }, { name: command, failureMessage: message });
}

describe("B172 write result and collaboration read-model audit", function () {
  this.timeout(90000);

  before(async () => {
    const base = await buildPageFixtures(browser.options.baseUrl);
    const invite = {
      id: 9101,
      project_id: 1,
      project_title: "协作邀请项目",
      inviter_username: "另一位作者",
      invitee_username: "写作者",
      role: "editor",
      status: "pending",
      created_at: "2026-09-11T12:00:00Z",
      expires_at: null,
    };
    fixtures = {
      ...base,
      list_my_project_invites: [invite],
      list_project_members: [
        { user_id: 1, username: "写作者", email: "writer@example.test", role: "owner" },
        { user_id: 2, username: "协作者", email: "collab@example.test", role: "editor" },
      ],
      list_project_invites: [invite],
    };
    await installUiFixtures(browser, fixtures);
    auditWindow = await browser.getWindowHandle();
  });

  it("UX-1949 reports project creation before a project-list refresh completes", async () => {
    const scope = await home();
    const created = { ...fixtures.projectRecords[0], id: 101, title: "新建测试项目" };
    await browser.execute((value) => { window.__uiAudit.fixtures.create_project = value; }, created);
    await defer("create_project");
    await clickButton("创建项目", scope);
    const creation = await activeDialog("新建项目");
    await clickButton("自由创作", creation);
    await pending("create_project");
    await browser.execute(() => document.querySelector('button[aria-label="打开设置"]')?.click());
    const guard = await activeDialog("未保存修改");
    assert.match(await guard.getText(), /操作仍在进行/);
    await clickButton("取消", guard);
    await setFailure("list_projects", "项目目录刷新失败");
    await respond("create_project", created);
    await browser.waitUntil(async () => (await $("body").getText()).includes("已创建：新建测试项目"), { timeout });
  });

  it("UX-1953 keeps delete feedback when the follow-up project read fails", async () => {
    const scope = await home();
    const project = fixtures.projectRecords.find((item) => item.id === 2);
    const row = await scope.$(`.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=${literal(project.title)}]]`);
    await row.$('button[aria-label="更多操作"]').click();
    await clickButton("删除项目");
    const confirmation = await activeDialog("删除项目");
    await defer("delete_project");
    await clickButton("永久删除", confirmation);
    await pending("delete_project");
    await browser.execute(() => document.querySelector('button[aria-label="打开设置"]')?.click());
    const guard = await activeDialog("未保存修改");
    assert.match(await guard.getText(), /操作仍在进行/);
    await clickButton("取消", guard);
    await setFailure("list_projects", "删除后的项目目录刷新失败");
    await respond("delete_project", null);
    await browser.waitUntil(async () => (await $("body").getText()).includes(`已删除：${project.title}`), { timeout });
  });

  it("UX-1955 acknowledges an accepted invitation before project reads settle", async () => {
    const scope = await home();
    await browser.waitUntil(async () => (await scope.getText()).includes("待处理项目邀请"), { timeout });
    await defer("accept_project_invite");
    await clickButton("接受", scope);
    await pending("accept_project_invite");
    await browser.execute(() => document.querySelector('button[aria-label="打开设置"]')?.click());
    const guard = await activeDialog("未保存修改");
    assert.match(await guard.getText(), /操作仍在进行/);
    await clickButton("取消", guard);
    await setFailure("list_projects", "接受邀请后的项目目录刷新失败");
    await respond("accept_project_invite", null);
    await browser.waitUntil(async () => (await $("body").getText()).includes("已接受项目邀请"), { timeout });
  });

  it("UX-1957 keeps collaboration invitation feedback attached to the modal", async () => {
    await home();
    const projectRow = await $(
      `.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=${literal(fixtures.projectRecords[0].title)}]]`,
    );
    await projectRow.$("button").click();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/novel/1", { timeout });
    await clickButton("更多");
    const actions = await activeDialog("项目操作");
    await clickButton("协作成员", actions);
    const collaboration = await activeDialog("协作成员");
    await collaboration.$('input[aria-label="用户名或邮箱"]').setValue("new-collaborator@example.test");
    await defer("invite_project_member");
    await clickButton("邀请", collaboration);
    await pending("invite_project_member");
    await browser.keys(["ESC"]);
    await browser.waitUntil(async () => (await $("body").getText()).includes("操作仍在进行"), { timeout });
    const invite = { ...fixtures.list_project_invites[0], id: 9201, invitee_username: "new-collaborator" };
    await setFailure("list_project_members", "成员目录刷新失败");
    await respond("invite_project_member", invite);
    await browser.waitUntil(async () => (await $("body").getText()).includes("邀请已发送"), { timeout });
  });

  it("UX-1962 clears the member snapshot when the scoped read fails", async () => {
    await home();
    const projectRow = await $(
      `.//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=${literal(fixtures.projectRecords[0].title)}]]`,
    );
    await projectRow.$("button").click();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/novel/1", { timeout });
    await clickButton("更多");
    const actions = await activeDialog("项目操作");
    await clickButton("协作成员", actions);
    const collaboration = await activeDialog("协作成员");
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "list_project_members", status: 503, message: "成员目录暂时不可用",
    }));
    await browser.keys(["ESC"]);
    await clickButton("更多");
    const reopenedActions = await activeDialog("项目操作");
    await clickButton("协作成员", reopenedActions);
    const failed = await activeDialog("协作成员");
    await browser.waitUntil(async () => (await failed.getText()).includes("成员目录暂时不可用"), { timeout });
    assert.equal((await failed.getText()).includes("协作者"), false);
  });
});
