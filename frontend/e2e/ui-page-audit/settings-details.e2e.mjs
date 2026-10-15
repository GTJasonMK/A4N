import assert from "node:assert/strict";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";

const seed = JSON.parse(await readFile(new URL("./settings-fixtures.json", import.meta.url), "utf8"));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";
const runId = new Date().toISOString().replace(/[:.]/g, "-");
const output = fileURLToPath(new URL(
  `../../../.dev-runtime/ui-pages/settings-details/${phase}-${runId}/`,
  import.meta.url,
));
const widths = [1440, 1024, 900, 390];
const minPromptEditorHeight = 240;
const report = { phase, runId, setupComplete: false, visualReview: "pending", planned: [], screenshots: [], tests: [] };
const prompt = seed.invoke.prompt_list.find((item) => item.name === "blueprint");
const secondaryPrompt = seed.invoke.prompt_list.find((item) => item.name !== prompt.name);
const editableTheme = seed.invoke.list_themes.find((item) => item.id === 13);
const fakePassword = "UI-audit-only-Password-2026";
let trace = [];
let currentId = "";
let primaryWindow;
let failureMessage = null;

const step = (action, details = {}) => { trace.push({ action, ...details }); };
const workflow = (title, body) => it(title, async function () {
  try {
    await body();
  } catch (error) {
    failureMessage = String(error?.message ?? error);
    throw error;
  }
});
const button = (text) => $(`//button[normalize-space()=${JSON.stringify(text)}]`);
const summary = (text) => $(`//summary[contains(normalize-space(),${JSON.stringify(text)})]`);
const dialog = (title) => $(`//div[@role="dialog"][.//h3[normalize-space()=${JSON.stringify(title)}]]`);
const menuItem = (text) => $(`//*[@role="menuitem" and normalize-space()=${JSON.stringify(text)}]`);
const section = (label) => $(`section[aria-label="${label}"]`);

async function click(element, action) {
  await element.waitForDisplayed();
  await element.click();
  step(action);
}

async function openDetails(text) {
  const control = await summary(text);
  const open = await control.execute((element) => element.parentElement.open);
  if (!open) await click(control, `展开 ${text}`);
  assert.equal(await control.execute((element) => element.parentElement.open), true);
}

async function choose(control, label) {
  await click(control, `打开选择器：${label}`);
  await click(await $(`//*[@role="option" and normalize-space()=${JSON.stringify(label)}]`), `选择 ${label}`);
  await $('[role="listbox"]').waitForExist({ reverse: true });
}

async function inputFor(label) {
  const text = JSON.stringify(label);
  return $(`//label[normalize-space()=${text} or span[normalize-space()=${text}]]//*[self::input or self::textarea or self::button[@role="combobox"]] | //label[normalize-space()=${text}]/following-sibling::*[self::input or self::textarea or self::button[@role="combobox"]]`);
}

async function rowWith(text) {
  await browser.waitUntil(async () => {
    for (const row of await $$(".a4n-settings-content li")) {
      if ((await row.getText()).includes(text)) return true;
    }
    return false;
  }, { timeout: 5000, timeoutMsg: `Missing settings row: ${text}` });
  const rows = await $$(".a4n-settings-content li");
  for (const row of rows) if ((await row.getText()).includes(text)) return row;
  throw new Error(`Missing settings row: ${text}`);
}

async function openMenu(row) {
  await click(await row.$('button[aria-haspopup="menu"]'), "打开当前行操作菜单");
  await $('[role="menu"]').waitForDisplayed();
}

async function closeDialog(title) {
  await dialog(title).waitForDisplayed();
  await browser.keys("Escape");
  step(`Escape 关闭 ${title}`);
  await dialog(title).waitForExist({ reverse: true });
}

async function begin(tab) {
  await browser.switchToWindow(primaryWindow);
  await browser.switchFrame(null);
  await browser.setWindowSize(1440, 1000);
  if (await browser.isAlertOpen()) {
    step("确认上一条失败用例留下的原生离开提示");
    await browser.acceptAlert();
  }
  await browser.url("/");
  await $(".a4n-home-content").waitForDisplayed();
  await $('[role="dialog"]').waitForExist({ reverse: true });
  step("回到首页，确认前一流程文档与弹窗已销毁");
  await browser.url(`/settings#${tab}`);
  await $(".a4n-settings-content").waitForDisplayed();
  step("通过设置公开深链进入", { tab });
}

async function openPrompt() {
  await begin("prompt");
  const item = await $('.a4n-settings-content').$(`button*=${prompt.title}`);
  await click(item, `选择提示词 ${prompt.name}`);
  await $(`textarea[aria-label=${JSON.stringify(prompt.title)}]`).waitForDisplayed();
}

async function assertPromptEditorHeight(width) {
  const { height } = await $(`textarea[aria-label=${JSON.stringify(prompt.title)}]`).getSize();
  assert(height >= minPromptEditorHeight, `Primary prompt editor collapsed to ${height}px at ${width}px`);
}

async function openAdmin(label) {
  await begin("admin");
  await click(await $('.a4n-settings-content').$(`button=${label}`), `管理子页：${label}`);
}

async function setFixtures(patch) {
  await browser.execute((values) => { Object.assign(window.__uiAudit.fixtures, values); }, patch);
  step("设置明确的模拟读响应", { commands: Object.keys(patch) });
}

async function defer(command) {
  await browser.execute((name) => {
    if (!window.__uiAudit.deferred.includes(name)) window.__uiAudit.deferred.push(name);
  }, command);
  step("安装显式请求钩子", { command });
}

async function pending(command, match = {}) {
  await browser.waitUntil(async () => browser.execute((name, fields) =>
    window.__uiAudit.pending.some((request) => request.command === name &&
      Object.entries(fields).every(([key, value]) =>
        (request.body[key] ?? request.body.input?.[key]) === value)),
  command, match), { timeout: 5000, timeoutMsg: `Expected an actual UI request: ${command}` });
  return browser.execute((name, fields) => {
    const request = window.__uiAudit.pending.find((entry) => entry.command === name &&
      Object.entries(fields).every(([key, value]) =>
        (entry.body[key] ?? entry.body.input?.[key]) === value));
    return request.body;
  }, command, match);
}

async function respond(command, body, { fixtures = {}, status = 200, match = {} } = {}) {
  await browser.execute((name, response, values, httpStatus, fields) => {
    const audit = window.__uiAudit;
    const index = audit.pending.findIndex((entry) => entry.command === name &&
      Object.entries(fields).every(([key, value]) =>
        (entry.body[key] ?? entry.body.input?.[key]) === value));
    if (index < 0) throw new Error(`No pending request to settle: ${name}`);
    Object.assign(audit.fixtures, values);
    const [request] = audit.pending.splice(index, 1);
    request.resolve(Response.json(response, { status: httpStatus }));
  }, command, body, fixtures, status, match);
  step("显式响应已经发出的请求", { command, status, match });
}

async function pageState() {
  return browser.execute(() => {
    const audit = window.__uiAudit;
    const redact = (value) => {
      if (!value || typeof value !== "object") return value;
      if (Array.isArray(value)) return value.map(redact);
      return Object.fromEntries(Object.entries(value).map(([key, child]) => [
        key,
        /password|token|api.?key|secret|code|verifier|challenge|state/i.test(key)
          ? "[redacted]"
          : redact(child),
      ]));
    };
    const url = new URL(location.href);
    const focused = document.activeElement;
    return {
      url: url.origin + url.pathname + url.hash,
      fixtureInstalled: Boolean(audit),
      width: innerWidth,
      height: innerHeight,
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      text: document.body.innerText,
      focus: focused ? { tag: focused.tagName, label: focused.getAttribute("aria-label"), type: focused.getAttribute("type") } : null,
      dialogs: [...document.querySelectorAll('[role="dialog"]')].map((element) => {
        const rect = element.getBoundingClientRect();
        return { title: document.getElementById(element.getAttribute("aria-labelledby"))?.textContent, width: rect.width, height: rect.height };
      }),
      requests: redact(audit?.requests),
      pending: audit?.pending.map((request) => request.command),
      unexpected: audit?.unexpected,
      errors: audit?.errors,
    };
  });
}

async function capture(state, { anchor, prepare, verify, viewports = widths } = {}) {
  for (const width of viewports) {
    await browser.setWindowSize(width, 1000);
    await browser.pause(200);
    if (prepare) await prepare(width);
    if (anchor) {
      const target = await $(anchor);
      await target.waitForDisplayed();
      await target.execute((element) => element.scrollIntoView({ block: "start", inline: "nearest" }));
      const size = await target.getSize();
      assert(size.width > 1 && size.height > 1, `Collapsed required surface: ${anchor}`);
    }
    await browser.pause(200);
    const snapshot = await pageState();
    const file = `${currentId}-${state}-${width}.png`;
    await browser.saveScreenshot(`${output}/${file}`);
    report.screenshots.push({ id: currentId, state, file, trace: [...trace], ...snapshot });
    assert.equal(snapshot.fixtureInstalled, true);
    assert.equal(snapshot.width, width);
    assert.equal(snapshot.overflow, 0);
    assert.deepEqual(snapshot.unexpected, []);
    assert.deepEqual(snapshot.errors, []);
    if (verify) await verify(width);
  }
  await browser.setWindowSize(1440, 1000);
  await browser.pause(200);
}

async function install(overrides = {}) {
  const data = JSON.parse(JSON.stringify({
    ...visualFixtures,
    ...seed.invoke,
    pageSessions: seed.rest.pageSessions,
    oauth2_available_providers: ["google"],
    ...overrides,
  }), (_key, value) =>
    typeof value === "string"
      ? value.replace(/https:\/\/ui\.invalid\/image-(\d+)\.png/g, (_match, number) => `${browser.options.baseUrl}/e2e/ui-page-audit/media/image-${number}.svg`)
      : value);
  await installUiFixtures(browser, data);
}

describe("Settings detail pages and complete dialog workflows", function () {
  this.timeout(120000);

  before(async () => {
    await mkdir(output, { recursive: true });
    primaryWindow = await browser.getWindowHandle();
    await install();
    await browser.url("/");
    await $(".a4n-home-content").waitForDisplayed();
    await browser.execute(async (models) => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      for (const input of models.llm) await local.createLocalLlmConfig(input);
      for (const input of models.image) await local.createLocalImageConfig(input);
      for (const input of models.video) await local.createLocalVideoConfig(input);
      let firstEmbedding;
      for (const input of models.embedding) {
        const item = await local.createLocalEmbeddingConfig(input);
        firstEmbedding ??= item.id;
      }
      await local.setDefaultLocalEmbeddingConfig(firstEmbedding);
      await local.updateLocalWebToolsConfig(models.webTools.search, models.webTools.fetch);
    }, seed.localModels);
    await browser.execute((sessionId) => {
      localStorage.setItem("a4n:current_session_id", sessionId);
    }, seed.invoke.list_sessions[0].id);
    report.setupComplete = true;
    console.info(`Settings detail evidence: ${output}`);
  });

  after(async () => {
    await writeFile(`${output}/settings-details-report.json`, JSON.stringify(report, null, 2));
  });

  beforeEach(function () {
    currentId = this.currentTest.title.split(" ")[0];
    trace = [];
    failureMessage = null;
  });

  afterEach(async function () {
    const record = { id: currentId, title: this.currentTest.title, status: this.currentTest.state, trace: [...trace] };
    const snapshot = await pageState();
    const unsettled = this.currentTest.state === "passed" && snapshot.pending?.length > 0;
    if (this.currentTest.state === "failed" || unsettled) {
      record.status = "failed";
      const file = `${currentId}-failure.png`;
      await browser.saveScreenshot(`${output}/${file}`);
      record.failure = {
        message: unsettled ? "The case left an explicit fixture request unresolved" : failureMessage ?? this.currentTest.err?.message,
        file, ...snapshot,
      };
    }
    report.tests.push(record);
    await writeFile(`${output}/settings-details-report.json`, JSON.stringify(report, null, 2));
    assert.equal(unsettled, false, "Do not count an unresolved fixture request as a completed workflow");
  });

  workflow("SD01-prompt-editor edits, saves and protects a second draft", async () => {
    await openPrompt();
    await openDetails("高级工具");
    await openDetails("依赖图 / 反向依赖");
    const directions = await browser.execute(() => {
      const panel = [...document.querySelectorAll("details")].find((item) =>
        item.querySelector("summary")?.textContent.includes("依赖图"),
      );
      const grid = panel?.querySelector(".grid");
      if (!grid) throw new Error("提示词依赖图没有展开");
      return [...grid.children].map((column) => {
        const label = column.firstElementChild;
        return {
          text: label?.textContent.trim(),
          direction: [...(label?.querySelectorAll("svg") ?? [])].map((icon) =>
            icon.classList.contains("lucide-arrow-right") ? "right"
              : icon.classList.contains("lucide-arrow-left") ? "left" : "other",
          ),
          iconsHidden: [...(label?.querySelectorAll("svg") ?? [])].every((icon) => icon.getAttribute("aria-hidden") === "true"),
        };
      });
    });
    assert.deepEqual(directions, [
      { text: "此模板依赖", direction: ["right"], iconsHidden: true },
      { text: "依赖此模板", direction: ["left"], iconsHidden: true },
    ]);
    await capture("dependency-directions", { anchor: '//summary[contains(normalize-space(),"依赖图")]' });
    await capture("selected-content", {
      anchor: `textarea[aria-label=${JSON.stringify(prompt.title)}]`,
      verify: assertPromptEditorHeight,
    });
    const editor = await $(`textarea[aria-label=${JSON.stringify(prompt.title)}]`);
    const content = prompt.content + "\n\n界面审查：保留原文证据，再推进人物选择。";
    await editor.setValue(content);
    step("修改正文，等待保存");
    await capture("dirty-content", { anchor: `textarea[aria-label=${JSON.stringify(prompt.title)}]` });
    await defer("prompt_update");
    await click(await button("保存"), "提交提示词正文");
    const request = await pending("prompt_update");
    assert.deepEqual(request.input, { name: prompt.name, content });
    const saved = { ...prompt, content, is_modified: true, version: "ui-details-saved" };
    await respond("prompt_update", saved, {
      fixtures: { prompt_list: seed.invoke.prompt_list.map((item) => item.name === prompt.name ? saved : item) },
    });
    await browser.waitUntil(async () => !await browser.execute(() => Boolean(document.querySelector('[data-settings-dirty="true"]'))));
    assert.equal(await editor.getValue(), content);
    await capture("saved-content");
    await editor.setValue(content + "\n尚未保存的第二次修改。");
    await click(await $('.a4n-settings-content').$('button*=章节正文生成'), "尝试切换到另一模板");
    await dialog("未保存修改").waitForDisplayed();
    await capture("discard-dialog", {
      verify: async (width) => {
        const currentDialog = await dialog("未保存修改");
        const cancel = await currentDialog.$("button=取消");
        const confirm = await currentDialog.$("button=放弃草稿");
        const [cancelPosition, confirmPosition] = await Promise.all([
          cancel.getLocation(),
          confirm.getLocation(),
        ]);
        if (width < 640) {
          assert.ok(cancelPosition.y < confirmPosition.y, "Mobile warning actions must follow the keyboard order from top to bottom");
        } else {
          assert.ok(cancelPosition.x < confirmPosition.x, "Desktop warning actions must follow the keyboard order from left to right");
        }
        await cancel.execute((element) => element.focus());
        await browser.keys("Tab");
        const focusedAction = await browser.execute(() => ({
          label: document.activeElement?.textContent?.trim(),
          visible: document.activeElement?.matches(":focus-visible"),
        }));
        assert.deepEqual(focusedAction, { label: "放弃草稿", visible: true });
        await browser.execute(() => {
          if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
        });
      },
    });
    await click(await dialog("未保存修改").$('button=取消'), "取消丢弃，保留草稿");
    await dialog("未保存修改").waitForExist({ reverse: true });
    assert((await editor.getValue()).includes("尚未保存的第二次修改"));
    await editor.setValue(content);

    const savedDuringLeave = content + "\n保存完成后继续离开。";
    await editor.setValue(savedDuringLeave);
    await defer("prompt_update");
    await click(await button("保存"), "提交离页期间仍在保存的提示词");
    const saveRequest = await pending("prompt_update");
    await click(await $("nav").$("button=高级配置"), "保存未完成时请求切换设置页");
    const operationDialog = await dialog("未保存修改");
    await operationDialog.waitForDisplayed();
    assert((await operationDialog.getText()).includes("操作仍在进行"));
    assert.equal(await operationDialog.$("button=放弃草稿").isEnabled(), false);
    await capture("save-blocked-navigation");
    await respond("prompt_update", { ...prompt, content: savedDuringLeave, is_modified: true, version: "ui-details-leave-save" }, {
      fixtures: { prompt_list: seed.invoke.prompt_list.map((item) => item.name === prompt.name
        ? { ...item, content: savedDuringLeave, is_modified: true, version: "ui-details-leave-save" }
        : item) },
    });
    await browser.waitUntil(async () => await browser.execute(() => location.hash === "#advanced"));
    await operationDialog.waitForExist({ reverse: true });
    step("保存成功后完成先前请求的设置页切换", { name: saveRequest.input.name });
    await capture("saved-departure");
    const toastCoversTitle = await browser.execute(() => {
      const toast = document.querySelector(".a4n-toast")?.getBoundingClientRect();
      const title = document.querySelector('section[aria-label="服务器配置备份"] h3')?.getBoundingClientRect();
      if (!toast || !title) throw new Error("Expected the save confirmation toast and advanced settings title");
      return toast.left < title.right && toast.right > title.left && toast.top < title.bottom && toast.bottom > title.top;
    });
    assert.equal(toastCoversTitle, false, "The mobile save confirmation must not cover the active page title");
  });

  workflow("SD02-prompt-history opens versions and compares the selected pair", async () => {
    await openPrompt();
    await openDetails("高级工具");
    await openDetails("版本历史");
    await $('.a4n-settings-content').$('button*=最新版本').waitForDisplayed();
    await capture("history", { anchor: '//summary[contains(normalize-space(),"版本历史")]' });
    await click(await $('.a4n-settings-content').$('button*=最新版本'), "选择最新版本");
    await click(await $('.a4n-settings-content').$('button*=历史版本 1'), "选择一个历史版本");
    await defer("prompt_version_diff");
    await click(await button("对比已选版本"), "对比选定版本");
    const request = await pending("prompt_version_diff");
    assert.equal(request.input.name, prompt.name);
    assert.deepEqual(request.input.version_a, seed.invoke.prompt_version_history[0].version);
    assert.deepEqual(request.input.version_b, seed.invoke.prompt_version_history[1].version);
    await respond("prompt_version_diff", seed.invoke.prompt_version_diff);
    await browser.waitUntil(async () => (await $('.a4n-settings-content').getText()).includes("ui-commit-003"));
    await capture("version-diff", { anchor: '//summary[contains(normalize-space(),"版本历史")]' });
  });

  workflow("SD03-prompt-override updates and removes only the selected project override", async () => {
    await openPrompt();
    await openDetails("高级工具");
    await openDetails("项目独立版本");
    await choose(await $('button[aria-label="选择使用独立提示词的项目"]'), seed.invoke.list_projects.items[0].title);
    await button("更新为当前内容").waitForDisplayed();
    await capture("existing-override", {
      anchor: '//summary[contains(normalize-space(),"项目独立版本")]',
      verify: assertPromptEditorHeight,
    });
    await defer("prompt_set_project_override");
    await click(await button("更新为当前内容"), "提交当前保存内容为项目独立版本");
    await dialog("更新项目独立版本").waitForDisplayed();
    await capture("override-confirmation");
    await click(await dialog("更新项目独立版本").$('button=更新'), "确认更新独立版本");
    const request = await pending("prompt_set_project_override");
    assert.deepEqual(request.input, { project_id: 1, name: prompt.name, content: prompt.content });
    const overrides = seed.invoke.prompt_list_project_overrides.map((item) =>
      item.prompt_name === prompt.name ? { ...item, updated_at: "2026-09-25T09:00:00Z" } : item);
    await respond("prompt_set_project_override", null, { fixtures: { prompt_list_project_overrides: overrides } });
    await button("更新为当前内容").waitForEnabled();
    const block = await summary("项目独立版本").parentElement();
    await openMenu(block);
    await click(await menuItem("恢复全局内容"), "恢复所选项目的全局提示词");
    await dialog("移除项目独立版本").waitForDisplayed();
    await capture("remove-override-confirmation");
    await defer("prompt_remove_project_override");
    await click(await dialog("移除项目独立版本").$('button=移除'), "确认移除项目独立版本");
    const remove = await pending("prompt_remove_project_override");
    assert.deepEqual(remove.input, { project_id: 1, name: prompt.name });
    await respond("prompt_remove_project_override", null, {
      fixtures: { prompt_list_project_overrides: overrides.filter((item) => item.prompt_name !== prompt.name) },
    });
    await button("使用当前内容").waitForDisplayed();
    await capture("global-restored", { anchor: '//summary[contains(normalize-space(),"项目独立版本")]' });
  });

  workflow("SD04-prompt-validation exposes missing variables and renders the saved template", async () => {
    await openPrompt();
    await openDetails("高级工具");
    await openDetails("变量校验");
    const variables = await $('textarea[placeholder="如：title, chapter_number, character_name"]');
    await variables.setValue("project_title,chapter_summary");
    await click(await button("校验"), "检查不完整的变量列表");
    await browser.waitUntil(async () => (await $('.a4n-settings-content').getText()).includes("缺失:") || (await $('.a4n-settings-content').getText()).includes("缺失："));
    await capture("missing-variables", {
      anchor: '//summary[contains(normalize-space(),"变量校验")]',
      verify: assertPromptEditorHeight,
    });
    await setFixtures({ prompt_validate_variables: [] });
    await variables.setValue("project_title,chapter_summary,protagonist_goal");
    await click(await button("校验"), "检查完整变量列表");
    await browser.waitUntil(async () => (await summary("变量校验").parentElement().getText()).includes("所有占位符已覆盖"));
    await openDetails("渲染对照");
    await choose(await $('button[aria-label="渲染时使用的项目上下文"]'), seed.invoke.list_projects.items[0].title);
    const renderBlock = await summary("渲染对照").parentElement();
    const values = { project_title: "雾港来信", chapter_summary: "主角进入旧船坞寻找证据。", protagonist_goal: "查明失踪航线。" };
    await renderBlock.$("textarea").setValue(JSON.stringify(values, null, 2));
    await defer("prompt_render");
    await click(await button("直接渲染"), "渲染已保存模板");
    const direct = await pending("prompt_render");
    assert.deepEqual(direct.input, { project_id: 1, template_name: prompt.name, variables: values });
    await respond("prompt_render", seed.invoke.prompt_render);
    await renderBlock.$("pre").waitForDisplayed();
    await capture("direct-render", { anchor: '//summary[contains(normalize-space(),"渲染对照")]' });
    await defer("prompt_render_ab");
    await click(await button("当前/原始对照"), "比较当前保存版与原始模板");
    const comparison = await pending("prompt_render_ab");
    assert.deepEqual(comparison.input, direct.input);
    await respond("prompt_render_ab", seed.invoke.prompt_render_ab);
    await browser.waitUntil(async () => (await renderBlock.getText()).includes("原始模板："));
    await capture("render-comparison", { anchor: '//summary[contains(normalize-space(),"渲染对照")]' });
  });

  workflow("SD28-prompt-list uses immediate selection scrolling for reduced motion", async () => {
    await openPrompt();
    try {
      await capture("desktop-default", { viewports: [1440] });
      await browser.setWindowSize(390, 1000);
      await browser.sendCommandAndGetResult("Emulation.setEmulatedMedia", {
        features: [{ name: "prefers-reduced-motion", value: "reduce" }],
      });
      await browser.waitUntil(async () => browser.execute(() => Boolean(document.querySelector("[data-prompt-scroll-hint]"))));
      const listAtStart = await browser.execute(() => {
        const list = document.querySelector("[data-prompt-list]");
        if (!(list instanceof HTMLElement)) throw new Error("提示词列表不可用");
        return { height: list.clientHeight, scrollHeight: list.scrollHeight };
      });
      assert.ok(listAtStart.scrollHeight > listAtStart.height, "The prompt list must expose its overflowing state");
      await browser.execute(() => {
        const list = document.querySelector("[data-prompt-list]");
        if (!(list instanceof HTMLElement)) throw new Error("提示词列表不可用");
        list.scrollTo({ top: list.scrollHeight, behavior: "auto" });
      });
      await browser.waitUntil(async () => browser.execute(() => !document.querySelector("[data-prompt-scroll-hint]")));
      const hintAtEnd = await browser.execute(() => Boolean(document.querySelector("[data-prompt-scroll-hint]")));
      assert.equal(hintAtEnd, false, "The scroll hint must disappear at the end of the prompt list");
      await browser.execute(() => {
        const list = document.querySelector("[data-prompt-list]");
        if (!(list instanceof HTMLElement)) throw new Error("提示词列表不可用");
        list.scrollTo({ top: 0, behavior: "auto" });
      });
      await browser.waitUntil(async () => browser.execute(() => Boolean(document.querySelector("[data-prompt-scroll-hint]"))));
      step("列表仍有内容时显示提示，滚到底收起，再回顶恢复", {
        scrollHeight: listAtStart.scrollHeight,
        clientHeight: listAtStart.height,
      });
      await browser.execute(() => {
        const audit = window.__uiAudit;
        audit.selectedScrollBehaviors = [];
        audit.originalScrollIntoView = Element.prototype.scrollIntoView;
        Element.prototype.scrollIntoView = function (options) {
          if (this.matches('.a4n-settings-content button[aria-current="true"]')) {
            audit.selectedScrollBehaviors.push(options?.behavior ?? "auto");
          }
          return audit.originalScrollIntoView.call(this, options);
        };
      });

      await click(
        await $('.a4n-settings-content').$(`button*=${secondaryPrompt.title}`),
        `选择提示词 ${secondaryPrompt.name}`,
      );
      await browser.waitUntil(async () => browser.execute((title) => {
        const selected = document.querySelector('.a4n-settings-content button[aria-current="true"]');
        return Boolean(selected?.textContent?.includes(title) && window.__uiAudit.selectedScrollBehaviors.length > 0);
      }, secondaryPrompt.title), { timeoutMsg: "The selected prompt did not scroll into view" });
      const state = await browser.execute(() => ({
        reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
        behavior: window.__uiAudit.selectedScrollBehaviors.at(-1),
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      }));
      assert.equal(state.reducedMotion, true);
      assert.equal(state.behavior, "auto");
      assert.equal(state.overflow, 0);
      step("选择提示词后按系统偏好即时滚动", { name: secondaryPrompt.name, behavior: state.behavior });
      await capture("selected-reduced-motion", { viewports: [390] });
    } finally {
      await browser.execute(() => {
        const audit = window.__uiAudit;
        if (audit?.originalScrollIntoView) {
          Element.prototype.scrollIntoView = audit.originalScrollIntoView;
          delete audit.originalScrollIntoView;
        }
        delete audit?.selectedScrollBehaviors;
      });
      await browser.sendCommandAndGetResult("Emulation.setEmulatedMedia", { features: [] });
    }
  });

  workflow("SD29-prompt-assembly preserves list and editor layout", async () => {
    await begin("prompt-assembly");
    await $('.a4n-settings-content main h3').waitForDisplayed();
    await browser.waitUntil(async () => browser.execute(() => {
      const main = document.querySelector(".a4n-settings-content main");
      return [...(main?.querySelectorAll("label") ?? [])].some((label) =>
        label.textContent.trim().startsWith("模型调用"),
      );
    }));
    const inspectLayout = async () => browser.execute(() => {
      const list = document.querySelector("[data-prompt-list]");
      const editor = document.querySelector(".a4n-settings-content main");
      if (!(list instanceof HTMLElement) || !(editor instanceof HTMLElement)) {
        throw new Error("提示词装配列表或编辑区不可用");
      }
      const callField = [...editor.querySelectorAll("label")].find((label) =>
        label.textContent.trim().startsWith("模型调用"),
      );
      if (!callField) throw new Error("提示词装配的模型调用字段不可用");
      const listRect = list.getBoundingClientRect();
      const editorRect = editor.getBoundingClientRect();
      const callControlRect = callField.getBoundingClientRect();
      return {
        width: innerWidth,
        viewportHeight: innerHeight,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        list: { left: listRect.left, right: listRect.right, bottom: listRect.bottom },
        editor: { left: editorRect.left, top: editorRect.top, width: editorRect.width },
        callControlBottom: callControlRect.bottom,
        listScrollHeight: list.scrollHeight,
        listClientHeight: list.clientHeight,
        scrollHintVisible: Boolean(document.querySelector("[data-prompt-scroll-hint]")),
      };
    });

    await capture("assembly-desktop", {
      viewports: [1440],
      verify: async () => {
        const layout = await inspectLayout();
        assert.ok(layout.list.right <= layout.editor.left, "Desktop prompt list must remain beside, not over, the editor");
        assert.ok(layout.editor.width > 300, "Desktop editor must retain usable width");
      },
    });
    await capture("assembly-mobile", {
      viewports: [390],
      verify: async () => {
        const layout = await inspectLayout();
        assert.equal(layout.width, 390);
        assert.equal(layout.overflow, 0);
        assert.ok(layout.editor.top > layout.list.bottom, "Mobile editor must follow the prompt list");
        assert.ok(layout.callControlBottom <= layout.viewportHeight, "The primary assembly control must enter the mobile first view");
        assert.ok(layout.listScrollHeight > layout.listClientHeight, "Mobile prompt list must remain scrollable");
        assert.equal(layout.scrollHintVisible, true, "Scrollable mobile prompt list must expose its continuation cue");
        step("收起模板列表后首屏显示模型调用控件", {
          callControlBottom: layout.callControlBottom,
          viewportHeight: layout.viewportHeight,
          listOverflow: layout.listScrollHeight - layout.listClientHeight,
          scrollHintVisible: layout.scrollHintVisible,
        });
      },
    });
  });

  workflow("SD05-theme-editor exposes every editor section and saves a real draft", async () => {
    await begin("theme");
    const row = await rowWith(editableTheme.name);
    const opener = await row.$('button[aria-label="编辑主题"]');
    await click(opener, "打开有背景资源的完整主题编辑器");
    await dialog(editableTheme.name).waitForDisplayed();
    await capture("editor-top", { anchor: "#theme-editor-title" });
    await capture("editor-colors", { anchor: "#theme-editor-colors" });
    await capture("editor-effects", { anchor: "#theme-editor-effects" });
    await click(await $('button[aria-label="页面范围"]'), "打开主题背景页面范围");
    await browser.keys(["Home", "ArrowDown", "ArrowDown", "ArrowDown", "ArrowDown", "Enter"]);
    step("用键盘选择写作桌背景范围");
    await $('[role="listbox"]').waitForExist({ reverse: true });
    await capture("editor-background", { anchor: "#theme-editor-background" });
    await openDetails("流式显示美化");
    await capture("editor-streaming", { anchor: '//summary[contains(normalize-space(),"流式显示美化")]' });
    await openDetails("高级 Token");
    await capture("editor-advanced-and-footer", { anchor: '//summary[contains(normalize-space(),"高级 Token")]' });
    const name = editableTheme.name + " · 校订";
    await (await inputFor("主题名称")).setValue(name);
    await defer("update_theme");
    await click(await dialog(editableTheme.name).$('button=保存'), "保存完整主题草稿");
    const request = await pending("update_theme", { themeId: editableTheme.id });
    assert.equal(request.input.name, name);
    assert.equal(request.input.document.version, editableTheme.document.version);
    assert.deepEqual(request.input.document.backgrounds, editableTheme.document.backgrounds);
    const saved = { ...editableTheme, ...request.input };
    await respond("update_theme", saved, {
      match: { themeId: editableTheme.id },
      fixtures: { list_themes: seed.invoke.list_themes.map((item) => item.id === saved.id ? saved : item) },
    });
    await dialog(name).waitForDisplayed();
    await browser.waitUntil(async () => !await browser.execute(() => Boolean(document.querySelector('[data-settings-dirty="true"]'))));
    await closeDialog(name);
    await browser.waitUntil(async () => opener.isFocused(), { timeout: 2500, timeoutMsg: "Closing the theme editor must restore focus to its opener" });
  });

  workflow("SD06-theme-create-import-package reviews the three distinct theme dialogs", async () => {
    await begin("theme");
    await click(await button("创建主题"), "打开新建主题");
    await dialog("创建主题").waitForDisplayed();
    const name = "界面审查 · 新主题";
    await (await inputFor("主题名称")).setValue(name);
    await capture("create-filled");
    await defer("create_theme");
    await click(await dialog("创建主题").$('button=创建主题'), "提交新主题");
    const request = await pending("create_theme");
    assert.equal(request.input.name, name);
    assert.equal(request.input.mode, "light");
    assert.equal(request.input.document.version, 1);
    const created = { ...request.input, id: 15, user_id: 1, is_active: false, is_system_default: false, resources: [] };
    await respond("create_theme", created, { fixtures: { list_themes: [...seed.invoke.list_themes, created] } });
    await browser.waitUntil(async () => (await (await inputFor("主题名称")).getValue()) === "");
    await closeDialog("创建主题");

    await click(await button("导入主题"), "打开 JSON 主题导入");
    await dialog("导入主题").waitForDisplayed();
    const importedInput = { name: "界面审查 · 导入主题", mode: "light", document: seed.invoke.list_themes[1].document };
    await $('textarea[aria-label="导入主题"]').setValue(JSON.stringify(importedInput, null, 2));
    await capture("import-filled");
    await defer("import_theme");
    await click(await dialog("导入主题").$('button=导入主题'), "提交主题导入");
    const importedRequest = await pending("import_theme");
    assert.deepEqual(importedRequest.input, importedInput);
    const imported = { ...importedInput, id: 16, user_id: 1, is_active: false, is_system_default: false, resources: [] };
    await respond("import_theme", imported, { fixtures: { list_themes: [...seed.invoke.list_themes, created, imported] } });
    await browser.waitUntil(async () => (await $('textarea[aria-label="导入主题"]').getValue()) === "");
    await closeDialog("导入主题");

    await click(await button("主题包"), "打开带资源的主题包工作流");
    await dialog("主题包").waitForDisplayed();
    await choose(await $("#theme-package-theme"), editableTheme.name);
    await capture("package-options");
    await click(await $("#theme-package-theme"), "检查完整主题名称");
    await capture("package-theme-options", {
      verify: async (width) => {
        const layout = await browser.execute(() => {
          const trigger = document.querySelector("#theme-package-theme");
          const listbox = document.querySelector('[role="listbox"]');
          const selected = listbox?.querySelector('[role="option"][aria-selected="true"]');
          const label = selected?.querySelector(":scope > span");
          const range = document.createRange();
          if (label) range.selectNodeContents(label);
          const bounds = listbox?.getBoundingClientRect();
          return {
            triggerWidth: trigger?.getBoundingClientRect().width ?? 0,
            left: bounds?.left ?? null,
            right: bounds?.right ?? null,
            width: bounds?.width ?? 0,
            optionText: label?.textContent?.trim() ?? null,
            optionLines: range.getClientRects().length,
          };
        });
        assert.equal(layout.optionText, editableTheme.name);
        assert.equal(
          layout.optionLines,
          1,
          `The selected theme name must stay on one line at ${width}px`,
        );
        assert.ok(
          layout.width >= layout.triggerWidth,
          `The option popup must fit the trigger at ${width}px`,
        );
        assert.ok(
          layout.left >= 0 && layout.right <= width,
          `The option popup must stay inside ${width}px viewport`,
        );
        step("完整主题名称单行显示，选项弹层未越界", { width, ...layout });
      },
    });
    const clippedNames = await browser.execute(() => [...document.querySelectorAll('[role="option"] > span')]
      .filter((element) => element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 1)
      .map((element) => element.textContent));
    assert.deepEqual(clippedNames, [], "Theme choices must expose complete names");
    await browser.keys("Escape");
    step("ZIP 文件选择与二进制传输未执行；这里只审已填充的导入导出工作面");
    await closeDialog("主题包");
  });

  workflow("SD07-theme-more-rename-bulk preserves built-in and active themes", async () => {
    await begin("theme");
    let row = await rowWith(editableTheme.name);
    await openMenu(row);
    await capture("theme-more-menu");
    await browser.keys("Escape");
    await $('[role="menu"]').waitForExist({ reverse: true });
    await click(await row.$(`button=${editableTheme.name}`), "进入主题重命名");
    const newName = "雾港背景 · 重命名样本";
    await $('input[aria-label="重命名"]').setValue(newName);
    await capture("rename-draft");
    await defer("update_theme");
    await browser.keys("Enter");
    step("Enter 提交重命名");
    const request = await pending("update_theme", { themeId: editableTheme.id });
    assert.deepEqual(request.input, { name: newName });
    const renamed = { ...editableTheme, name: newName };
    await respond("update_theme", renamed, {
      match: { themeId: editableTheme.id },
      fixtures: { list_themes: seed.invoke.list_themes.map((item) => item.id === renamed.id ? renamed : item) },
    });
    await $('input[aria-label="重命名"]').waitForExist({ reverse: true });
    const renameFeedback = await $('[data-theme-rename-feedback]');
    await renameFeedback.waitForDisplayed();
    assert.equal((await renameFeedback.getText()).trim(), "主题已重命名");
    step("重命名成功状态以内联方式显示");
    await capture("rename-feedback-inline", {
      anchor: "[data-theme-rename-feedback]",
      viewports: [390],
      verify: async () => {
        const overlapsThemeCard = await browser.execute(() => {
          const notice = document.querySelector("[data-theme-rename-feedback]")?.getBoundingClientRect();
          if (!notice) return true;
          return [...document.querySelectorAll(".a4n-settings-content li")].some((card) => {
            const rect = card.getBoundingClientRect();
            return notice.left < rect.right && notice.right > rect.left
              && notice.top < rect.bottom && notice.bottom > rect.top;
          });
        });
        assert.equal(overlapsThemeCard, false, "Rename feedback must not cover theme cards");
        assert.equal(await browser.execute(() => [...document.querySelectorAll(".a4n-toast")]
          .some((toast) => toast.textContent?.includes("主题已重命名"))), false);
      },
    });
    await click(await button("全选"), "选择允许批量删除的两个主题");
    const checked = await $$('input[type="checkbox"]:checked');
    assert.equal(checked.length, 2);
    await capture("bulk-selection");
    await click(await button("删除选中（2）"), "打开批量删除确认");
    const confirmation = await dialog("删除选中（2）");
    await confirmation.waitForDisplayed();
    await capture("bulk-confirmation", {
      verify: async (width) => {
        const message = await confirmation.getText();
        assert(message.includes(newName));
        assert(message.includes("深木书房 · 夜间校稿"));
        assert(message.includes("不可撤销"));
        const cancelButton = await confirmation.$('button=取消');
        const deleteButton = await confirmation.$('button=删除选中（2）');
        const cancelPosition = await cancelButton.getLocation();
        const deletePosition = await deleteButton.getLocation();
        if (width < 640) assert(cancelPosition.y < deletePosition.y, "Mobile must place cancel above bulk deletion");
        else assert(cancelPosition.x < deletePosition.x, "Desktop must place cancel before bulk deletion");
      },
    });
    const cancelButton = await confirmation.$('button=取消');
    await click(cancelButton, "取消批量删除");
    await confirmation.waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "delete_theme").length), 0);
    await defer("delete_theme");
    await click(await button("删除选中（2）"), "再次打开批量删除");
    await click(await dialog("删除选中（2）").$('button=删除选中（2）'), "确认删除两个可删除主题");
    let remaining = seed.invoke.list_themes.map((item) => item.id === renamed.id ? renamed : item);
    for (const themeId of [13, 14]) {
      const deletion = await pending("delete_theme", { themeId });
      assert.equal(deletion.themeId, themeId);
      remaining = remaining.filter((item) => item.id !== themeId);
      await respond("delete_theme", null, { match: { themeId }, fixtures: { list_themes: remaining } });
    }
    await browser.waitUntil(async () => (await $('.a4n-settings-content').getText()).includes("2 个主题"));
    const retained = await browser.execute(() => window.__uiAudit.fixtures.list_themes.map((item) => item.id));
    assert.deepEqual(retained, [11, 12]);
    await capture("bulk-result");
  });

  workflow("SD08-admin-users opens users, invitations, token summary and usage records", async () => {
    await openAdmin("用户");
    await (await rowWith(seed.invoke.list_all_users[0].username)).waitForDisplayed();
    await openDetails("邀请码");
    await capture("users-and-invites", { anchor: '//summary[contains(normalize-space(),"邀请码")]' });
    const row = await rowWith(seed.invoke.list_all_users[0].username);
    await openMenu(row);
    await click(await menuItem("令牌摘要"), "打开用户令牌用量摘要");
    const tokenTitle = `${seed.invoke.list_all_users[0].username} · 令牌摘要`;
    await dialog(tokenTitle).waitForDisplayed();
    await capture("token-summary");
    await click(await dialog(tokenTitle).$('summary*=诊断'), "展开摘要原始数值");
    await capture("token-summary-details");
    await closeDialog(tokenTitle);
    await openMenu(row);
    await click(await menuItem("查看用量"), "打开用户近期调用");
    const usageTitle = `${seed.invoke.list_all_users[0].username} · 最近用量`;
    await dialog(usageTitle).waitForDisplayed();
    await capture("user-usage");
    await closeDialog(usageTitle);
  });

  workflow("SD09-admin-create-user submits a complete administrator-created account", async () => {
    await openAdmin("用户");
    await click(await button("新增用户"), "打开新增用户表单");
    await dialog("新增用户").waitForDisplayed();
    await (await inputFor("用户名")).setValue("界面审查作者");
    await (await inputFor("邮箱")).setValue("new-ui-author@example.invalid");
    await (await inputFor("初始密码")).setValue(fakePassword);
    await choose(await (await inputFor("角色")), "创作者");
    await capture("create-user-filled");
    await defer("create_user");
    await click(await dialog("新增用户").$('button=新增用户'), "提交新增用户");
    const request = await pending("create_user");
    assert.equal(request.input.username, "界面审查作者");
    assert.equal(request.input.email, "new-ui-author@example.invalid");
    assert.equal(request.input.role, "user");
    assert.equal(request.input.password, fakePassword);
    const created = {
      ...seed.invoke.list_all_users[2], id: 4, username: request.input.username,
      email: request.input.email, role: request.input.role,
      created_at: seed.fixtureDate, updated_at: seed.fixtureDate,
    };
    await respond("create_user", created, { fixtures: { list_all_users: [...seed.invoke.list_all_users, created] } });
    await dialog("新增用户").waitForExist({ reverse: true });
    await capture("created-user");
    assert((await (await rowWith(created.username)).getText()).includes("创作者"));
  });

  workflow("SD10-admin-role-disable-invite preserves explicit role and disable boundaries", async () => {
    await openAdmin("用户");
    const user = seed.invoke.list_all_users[1];
    let row = await rowWith(user.username);
    await openMenu(row);
    await click(await menuItem("调整角色"), "打开固定角色调整");
    await dialog("修改用户角色").waitForDisplayed();
    await capture("role-dialog");
    await $('input[aria-label="修改用户角色"]').setValue("管理员");
    await defer("update_user_role");
    await click(await dialog("修改用户角色").$('button=更新角色'), "提升为管理员");
    const role = await pending("update_user_role");
    assert.equal(role.userId, user.id);
    assert.equal(role.input.role, "admin");
    let users = seed.invoke.list_all_users.map((item) => item.id === user.id ? { ...item, role: "admin" } : item);
    await respond("update_user_role", users.find((item) => item.id === user.id), { fixtures: { list_all_users: users } });
    await browser.waitUntil(async () => (await (await rowWith(user.username)).getText()).includes("管理员"));
    row = await rowWith(user.username);
    await openMenu(row);
    await click(await menuItem("禁用用户"), "打开禁用用户确认");
    await dialog("禁用用户").waitForDisplayed();
    await capture("disable-confirmation");
    await defer("disable_user");
    await click(await dialog("禁用用户").$('button=禁用'), "确认禁用所选用户");
    const disabling = await pending("disable_user");
    assert.equal(disabling.userId, user.id);
    users = users.map((item) => item.id === user.id ? { ...item, is_active: false } : item);
    await respond("disable_user", null, { fixtures: { list_all_users: users } });
    await browser.waitUntil(async () => /禁用|停用/.test(await (await rowWith(user.username)).getText()));
    await click(await button("更多"), "打开用户管理工具");
    await click(await menuItem("新增邀请码"), "打开邀请码表单");
    await dialog("新增邀请码").waitForDisplayed();
    await $('input[aria-label="新增邀请码"]').setValue("UI-DETAILS-INVITE");
    await capture("invite-dialog");
    await defer("create_invite_code");
    await click(await dialog("新增邀请码").$('button=新增邀请码'), "提交测试邀请码");
    const invitation = await pending("create_invite_code");
    assert.deepEqual(invitation.input, { code: "UI-DETAILS-INVITE" });
    const created = { id: 13, code: invitation.input.code, created_at: seed.fixtureDate, created_by_user_id: 1, used_at: null, used_by_user_id: null };
    await respond("create_invite_code", created, { fixtures: { list_invite_codes: [...seed.invoke.list_invite_codes, created] } });
    await openDetails("邀请码");
    await capture("invite-created", { anchor: '//summary[contains(normalize-space(),"邀请码")]' });
  });

  workflow("SD11-admin-projects opens populated project ownership diagnostics", async () => {
    await openAdmin("项目");
    const row = await rowWith(seed.invoke.list_all_projects[0].title);
    await click(await row.$("summary"), "展开项目归属与标识");
    await capture("projects-with-diagnostics");
    assert((await row.getText()).includes("1"));
  });

  workflow("SD12-admin-tasks cancels exactly the selected queued task", async () => {
    await openAdmin("任务");
    await $('.a4n-settings-content li').waitForDisplayed();
    await capture("pending-tasks");
    const row = await $('.a4n-settings-content').$('li*=章节生成');
    await click(await row.$("summary"), "展开任务诊断");
    await openMenu(row);
    await capture("task-actions");
    await defer("cancel_admin_task");
    await click(await menuItem("取消任务"), "取消当前选中的管理员任务");
    const request = await pending("cancel_admin_task");
    assert.equal(request.taskId, seed.invoke.list_pending_tasks[0].id);
    await respond("cancel_admin_task", null, {
      fixtures: { list_pending_tasks: seed.invoke.list_pending_tasks.filter((item) => item.id !== request.taskId) },
    });
    await browser.waitUntil(async () => (await $$(".a4n-settings-content li")).length === seed.invoke.list_pending_tasks.length - 1);
    await capture("task-cancelled");
  });

  workflow("UX-695 keeps the admin section while its server config save is pending", async () => {
    await openAdmin("配置");
    const types = await $('[aria-label="API 配置类型"]');
    await click(await types.$("button=LLM 服务"), "打开服务器 LLM 配置");
    const config = seed.invoke.list_llm_config.find((item) => item.is_default);
    const row = await rowWith(config.name);
    await click(await row.$("button"), "选择默认服务器 LLM 配置");
    const name = await $(`input[value=${JSON.stringify(config.name)}]`);
    await name.setValue(config.name + " · 保存中");
    await defer("update_llm_config");
    await click(await $(".a4n-config-editor").$("button=保存"), "提交服务器配置变更");
    const request = await pending("update_llm_config");
    assert.equal(request.input.name, config.name + " · 保存中");

    const content = await $(".a4n-settings-content");
    await click(await content.$("button=用户"), "保存未完成时尝试切换管理员区块");
    await browser.waitUntil(async () => (await $(".a4n-toast").getText()).includes("操作仍在进行"));
    assert.equal(await content.$("button=配置").getAttribute("aria-pressed"), "true");
    assert.equal(await content.$("button=用户").getAttribute("aria-pressed"), "false");
    assert.equal(await dialog("未保存修改").isExisting(), false);
    await capture("admin-section-blocked");

    const updated = { ...config, name: request.input.name };
    await respond("update_llm_config", updated, {
      fixtures: { list_llm_config: seed.invoke.list_llm_config.map((item) => item.id === config.id ? updated : item) },
    });
    await browser.waitUntil(async () => await $(`input[value=${JSON.stringify(updated.name)}]`).isExisting());
    assert.equal(await content.$("button=配置").getAttribute("aria-pressed"), "true");
  });

  for (const [kind, label, command] of [
    ["llm", "LLM 服务", "list_llm_config"],
    ["image", "图片", "list_image_configs"],
    ["video", "视频", "list_video_configs"],
    ["embedding", "嵌入", "list_embedding_config"],
  ]) {
    workflow(`SD13-server-${kind} opens the server editor and its complete pricing fields`, async () => {
      await openAdmin("配置");
      await $('[aria-label="API 配置类型"]').waitForDisplayed();
      await click(await $('[aria-label="API 配置类型"]').$(`button=${label}`), `服务器配置：${kind}`);
      const config = seed.invoke[command].find((item) => item.is_default);
      const selectConfig = async () => {
        const selected = await rowWith(config.name);
        await click(await selected.$("button"), `选择服务器配置 ${config.id}`);
        await $(`input[value=${JSON.stringify(config.name)}]`).waitForDisplayed();
      };
      await capture("server-editor", { prepare: selectConfig });
      await capture("server-pricing", {
        prepare: async () => {
          await selectConfig();
          await openDetails("模型价格");
        },
        anchor: '//summary[contains(normalize-space(),"模型价格")]',
      });
      assert.equal(await $(`input[value=${JSON.stringify(config.name)}]`).getValue(), config.name);
      const requested = await browser.execute((name) => window.__uiAudit.commands.includes(name), command);
      assert.equal(requested, true, "The server editor must load its server command, not reuse local model storage");
    });
  }

  workflow("SD14-admin-trends keeps an initial failure visible and recovers on refresh", async () => {
    await begin("admin");
    await defer("get_usage_trends");
    await click(await $('.a4n-settings-content').$('button=趋势'), "打开尚未加载的趋势页");
    const initial = await pending("get_usage_trends");
    assert.equal(initial.input.days, 30);
    await respond("get_usage_trends", seed.variants.readFailure.body, { status: 503 });
    const error = await $('.a4n-settings-content [role="alert"]');
    await error.waitForDisplayed({ timeout: 5000 });
    assert((await error.getText()).includes(seed.variants.readFailure.body.message));
    await capture("initial-load-error", { anchor: '.a4n-settings-content [role="alert"]' });
    await click(await $('button[aria-label="刷新用量趋势"]'), "刷新失败的趋势请求");
    await pending("get_usage_trends");
    await respond("get_usage_trends", seed.invoke.get_usage_trends);
    await $('.a4n-settings-content [role="alert"]').waitForExist({ reverse: true });
    await browser.waitUntil(async () => (await $('.a4n-settings-content').getText()).includes("2026-09-25"));
    await capture("recovered-trends");
    await choose(await $('button[aria-label="统计范围"]'), "最近 7 天");
    const sevenDays = await pending("get_usage_trends");
    assert.equal(sevenDays.input.days, 7);
    await respond("get_usage_trends", seed.invoke.get_usage_trends.slice(-7));
    await browser.waitUntil(async () => (await $$(".a4n-settings-content li")).length === 7);
    await capture("seven-day-trends");
  });

  workflow("SD15-admin-packages distinguishes personal hiding from lifecycle rejection", async () => {
    await openAdmin("功能包");
    await $('[data-package-id="manga"]').waitForDisplayed();
    await capture("package-states");
    const hidden = await $('[data-package-id="manga"] input[data-package-preference]');
    assert.equal(await hidden.isSelected(), true);
    await defer("update_package_preferences");
    await click(hidden, "取消个人导航隐藏");
    const preference = await pending("update_package_preferences");
    assert.deepEqual(preference.input.hidden_package_ids, []);
    const savedPreference = { ...seed.invoke.get_package_preferences, hidden_package_ids: [] };
    await respond("update_package_preferences", savedPreference, {
      fixtures: { get_package_preferences: savedPreference },
    });
    await browser.waitUntil(async () => !(await hidden.isSelected()));
    await defer("start_package_lifecycle");
    await click(await $('[data-package-id="manga"] button[data-package-lifecycle]'), "请求启用漫画功能包");
    const lifecycle = await pending("start_package_lifecycle");
    assert.equal(lifecycle.input.package_id, "manga");
    assert.equal(lifecycle.input.action, "enable");
    assert.equal(lifecycle.input.expected_status_version, 8);
    await respond("start_package_lifecycle", {
      code: "package_unavailable", message: "状态已由另一位管理员更新，请刷新后重试。",
      retryable: false, trace_id: "ui-package-version-conflict",
      details: { package_id: "manga", reason_code: "version_conflict", status_version: 9 },
    }, { status: 409 });
    await $('[data-package-section] [role="alert"]').waitForDisplayed();
    await capture("lifecycle-error", { anchor: '[data-package-section] [role="alert"]' });
    assert.equal(await hidden.isSelected(), false, "A Host lifecycle rejection must not revert personal navigation preference");
  });

  workflow("SD16-runtime-logs expands paths and changes log level with an explicit response", async () => {
    await begin("logs");
    await click(await button("运行日志"), "进入运行日志子页");
    await $("#diagnostics-panel-logs pre").waitForDisplayed();
    await capture("log-content", { anchor: "#diagnostics-panel-logs" });
    await openDetails("日志级别");
    await openDetails("日志位置");
    await $('input[aria-label="日志过滤级别"]').setValue("info,a4n=trace");
    await capture("log-level-and-location", {
      anchor: '//summary[contains(normalize-space(),"日志级别")]',
      verify: async (width) => {
        const state = await browser.execute(() => {
          const region = document.querySelector("[data-log-scroll-region]");
          const hint = document.querySelector("[data-log-scroll-hint]");
          return {
            canScroll: Boolean(region && region.scrollHeight - region.clientHeight > 2),
            hintVisible: Boolean(hint && hint.getClientRects().length > 0),
          };
        });
        assert.equal(state.hintVisible, state.canScroll, `Log scroll hint must match overflow at ${width}px`);
        if (width === 390) assert.equal(state.canScroll, true, "The mobile log sample must exercise its internal scroll region");
      },
    });
    await capture("log-content-bottom", {
      anchor: "[data-log-scroll-region]",
      viewports: [390],
      prepare: async () => {
        await browser.execute(() => {
          const region = document.querySelector("[data-log-scroll-region]");
          if (region) region.scrollTop = region.scrollHeight;
        });
        await browser.waitUntil(async () => !await browser.execute(() => Boolean(document.querySelector("[data-log-scroll-hint]"))));
        step("滚动至日志末尾后收起继续滚动提示");
      },
      verify: async () => {
        const state = await browser.execute(() => {
          const rect = (element) => {
            if (!element) return null;
            const bounds = element.getBoundingClientRect();
            return { top: bounds.top, bottom: bounds.bottom };
          };
          const region = rect(document.querySelector("[data-log-scroll-region]"));
          const lastLine = rect(document.querySelector("[data-log-scroll-region] pre > div:last-child"));
          return { hintVisible: Boolean(document.querySelector("[data-log-scroll-hint]")), region, lastLine };
        });
        assert.equal(state.hintVisible, false);
        assert.ok(state.region && state.lastLine && state.lastLine.top >= state.region.top && state.lastLine.bottom <= state.region.bottom,
          "The last log line must be fully reachable inside the scroll region");
      },
    });
    await browser.execute(() => {
      const region = document.querySelector("[data-log-scroll-region]");
      if (region) region.scrollTop = 0;
    });
    await browser.setWindowSize(1440, 1000);
    await defer("set_log_level");
    await click(await button("应用级别"), "提交新的运行日志过滤级别");
    const request = await pending("set_log_level");
    assert.deepEqual(request.input, { filter: "info,a4n=trace" });
    await respond("set_log_level", null);
    await button("应用级别").waitForEnabled();
    await capture("log-level-applied", {
      anchor: '//summary[contains(normalize-space(),"日志级别")]',
      verify: async () => {
        const status = await $('//*[@role="status" and contains(normalize-space(.),"info,a4n=trace")]');
        assert.equal(await status.isDisplayed(), true);
        assert.equal(await $(".a4n-toast--success").isExisting(), false);
      },
    });
    await $('input[aria-label="日志过滤级别"]').setValue("info,a4n=debug");
    await $('//*[@role="status" and contains(normalize-space(.),"info,a4n=trace")]').waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "set_log_level").length), 1);
  });

  workflow("SD17-call-details opens every call domain and the full cost table", async () => {
    await begin("logs");
    for (const domain of ["llm-workflows", "media-workflows", "recent-llm", "recent-media"]) {
      const details = await $(`[data-diagnostic-domain="${domain}"]`);
      await click(await details.$("summary"), `展开调用域 ${domain}`);
      await capture(domain, { anchor: `[data-diagnostic-domain="${domain}"]` });
    }
    const cost = await $('section[aria-label="作品产出成本"]');
    await cost.waitForDisplayed();
    await capture("cost-table", { anchor: 'section[aria-label="作品产出成本"]' });
    await browser.setWindowSize(390, 1000);
    const table = await cost.$("table");
    await table.scrollIntoView({ block: "center" });
    const modelCell = await table.$("tbody tr:first-child td:nth-child(4)");
    await modelCell.execute((cell) => {
      const scroller = cell.closest("[role='region']");
      const stickyAmount = cell.parentElement?.firstElementChild;
      if (!scroller || !stickyAmount) throw new Error("The cost table needs its sticky amount column");
      scroller.scrollLeft += cell.getBoundingClientRect().left - stickyAmount.getBoundingClientRect().right;
    });
    const modelCellState = await modelCell.execute((cell) => {
      const scroller = cell.closest("[role='region']");
      const stickyAmount = cell.parentElement?.firstElementChild;
      const content = cell.querySelector("span")?.getBoundingClientRect();
      const box = cell.getBoundingClientRect();
      const viewport = scroller?.getBoundingClientRect();
      return {
        text: cell.textContent?.trim() ?? "",
        scrollWidth: cell.scrollWidth,
        clientWidth: cell.clientWidth,
        cell: { left: box.left, right: box.right, width: box.width },
        content: content ? { left: content.left, right: content.right } : null,
        stickyAmountRight: stickyAmount?.getBoundingClientRect().right ?? null,
        viewport: viewport ? { left: viewport.left, right: viewport.right } : null,
      };
    });
    assert.match(modelCellState.text, /openai.*ui-structured-model-long-context/);
    assert.ok(modelCellState.scrollWidth <= modelCellState.clientWidth + 1, "The mobile model cell must not clip its value");
    assert.ok(modelCellState.content.left >= modelCellState.stickyAmountRight - 1, "The sticky amount column must not cover the model value");
    assert.ok(modelCellState.content.right <= modelCellState.viewport.right + 1, "The model value must fit beside the sticky amount column");
    step("将完整 Provider / 模型单元格滚入手机视口", modelCellState);
    const modelScreenshot = `${currentId}-cost-model-390.png`;
    await browser.saveScreenshot(`${output}/${modelScreenshot}`);
    report.screenshots.push({ id: currentId, state: "cost-model", file: modelScreenshot, trace: [...trace], ...await pageState() });
    await table.parentElement().execute((element) => { element.scrollLeft = element.scrollWidth; });
    step("横向滚动成本表以查看金额和时间列");
    const file = `${currentId}-cost-table-right-390.png`;
    await browser.saveScreenshot(`${output}/${file}`);
    report.screenshots.push({ id: currentId, state: "cost-table-right", file, trace: [...trace], ...await pageState() });
    const llmText = await $('[data-diagnostic-domain="recent-llm"]').getText();
    assert.match(llmText, /失败/, "The two failed LLM records must be distinguishable from successful calls");
  });

  workflow("SD18-analytics-90-days preserves the full period and readable chart text", async () => {
    await begin("logs");
    await $('[data-diagnostic-domain="recent-llm"]').waitForDisplayed();
    await defer("get_generation_call_analytics");
    await choose(await $('button[aria-label="统计范围"]'), "最近 90 天");
    const request = await pending("get_generation_call_analytics");
    assert.equal(request.input.days, 90);
    const analytics = {
      ...seed.invoke.get_generation_call_analytics,
      ...seed.variants.analytics90Days,
      daily: [...seed.variants.analyticsOlderDaily, ...seed.invoke.get_generation_call_analytics.daily],
    };
    await respond("get_generation_call_analytics", analytics);
    const chart = await $('svg[aria-label="每日生成调用量"]');
    await browser.waitUntil(async () => (await chart.getText()).includes("08-12"));
    await capture("full-range-chart", {
      anchor: 'svg[aria-label="每日生成调用量"]',
      prepare: async () => {
        await chart.parentElement().execute((element) => { element.scrollLeft = 0; });
      },
    });
    for (const width of widths) {
      await browser.setWindowSize(width, 1000);
      await browser.pause(200);
      const labels = await chart.execute((element) => [...element.querySelectorAll("text")].map((label) => ({
        text: label.textContent, height: label.getBoundingClientRect().height,
      })));
      assert(labels.some((label) => label.text === "08-12"), "First day of the 45-day fixture must not be discarded");
      assert(labels.some((label) => label.text === "09-25"), "The latest date must remain available");
      assert(labels.every((label) => label.height >= 10), `Chart text shrank below 10 actual pixels at ${width}px`);
      await chart.parentElement().execute((element) => { element.scrollLeft = element.scrollWidth; });
      await chart.scrollIntoView({ block: "center", inline: "end" });
      step("检查图表末日与实际坐标字体", { width, labels });
      const file = `${currentId}-chart-last-date-${width}.png`;
      await browser.saveScreenshot(`${output}/${file}`);
      report.screenshots.push({ id: currentId, state: "chart-last-date", file, trace: [...trace], ...await pageState() });
    }
  });

  workflow("SD19-queue-trace exposes the final checkpoint and final failed generation call", async () => {
    await begin("queue");
    const runs = await $('//h3[normalize-space()="持久化工作流"]/ancestor::section[1]');
    await runs.waitForDisplayed();
    for (const width of widths) {
      await browser.setWindowSize(width, 1000);
      const actionRow = await runs.$("li*=章节分析");
      await actionRow.scrollIntoView({ block: "center", inline: "nearest" });
      await openMenu(actionRow);
      const context = await actionRow.execute((row) => {
        const trigger = row.querySelector('button[aria-haspopup="menu"]');
        const menu = document.querySelector('[role="menu"]');
        const rowBox = row.getBoundingClientRect();
        const triggerBox = trigger?.getBoundingClientRect();
        const menuBox = menu?.getBoundingClientRect();
        return {
          text: row.innerText,
          menuText: menu?.innerText ?? "",
          row: { top: rowBox.top, bottom: rowBox.bottom },
          trigger: triggerBox ? { top: triggerBox.top, bottom: triggerBox.bottom, right: triggerBox.right } : null,
          menu: menuBox ? { top: menuBox.top, bottom: menuBox.bottom, left: menuBox.left, right: menuBox.right } : null,
          viewport: { width: innerWidth, height: innerHeight },
        };
      });
      assert.match(context.text.replace(/\s+/g, " "), /失败.*可恢复|可恢复.*失败/, "The resume action must remain associated with the failed resumable run");
      assert.ok(context.menuText.includes("查看运行详情") && context.menuText.includes("继续运行"));
      assert.ok(context.row.top >= 0 && context.row.bottom <= context.viewport.height, "The action row must remain visible while its menu is open");
      assert.ok(context.menu && context.menu.top >= 0 && context.menu.bottom <= context.viewport.height);
      assert.ok(context.trigger && Math.abs(context.menu.right - context.trigger.right) <= 1, "The menu must stay aligned with its action trigger");
      step("在当前视口打开失败且可恢复工作流的菜单", { width, context });
      const file = `${currentId}-workflow-actions-${width}.png`;
      await browser.saveScreenshot(`${output}/${file}`);
      report.screenshots.push({ id: currentId, state: "workflow-actions", file, trace: [...trace], ...await pageState() });
      await browser.keys("Escape");
      await browser.waitUntil(async () => !(await $('[role="menu"]').isExisting()));
    }
    await browser.setWindowSize(1440, 1000);
    const row = await runs.$("li*=章节分析");
    await row.scrollIntoView({ block: "center", inline: "nearest" });
    await openMenu(row);
    await click(await menuItem("查看运行详情"), "展开第501次工作流的真实详情响应");
    await row.$("ol").waitForDisplayed();
    await capture("workflow-trace", { anchor: '//h3[normalize-space()="持久化工作流"]/ancestor::section[1]' });
    await click(await row.$("summary*=诊断"), "展开运行标识与调用诊断");
    await capture("workflow-diagnostics", { anchor: '//h3[normalize-space()="持久化工作流"]/ancestor::section[1]//li//summary[contains(normalize-space(),"诊断")]' });
    const lists = await row.$$("ol");
    assert.equal((await lists[0].$$("li")).length, seed.invoke.get_workflow_run_detail.checkpoints.length, "All checkpoints must be reachable, including the failed eighth step");
    assert.equal((await lists[1].$$("li")).length, seed.invoke.get_workflow_run_detail.generation_runs.length, "All generation calls must be reachable, including the failed tenth call");
    assert((await row.getText()).includes(seed.invoke.get_workflow_run_detail.generation_runs.at(-1).title));
    await browser.setWindowSize(390, 1000);
    const diagnosticStatuses = await browser.execute(() => {
      const element = [...document.querySelectorAll("li")].find((item) =>
        item.querySelector("details div.border-t") && item.querySelectorAll("ol").length >= 2
          && item.innerText.includes("第 7/8 项"),
      );
      if (!element) throw new Error("The expanded persistent workflow diagnostics are missing");
      const values = [...element.querySelectorAll("details div.border-t > div")];
      return values.flatMap((value) => {
        const textWalker = document.createTreeWalker(value, NodeFilter.SHOW_TEXT);
        const tokens = [];
        while (textWalker.nextNode()) {
          const text = textWalker.currentNode.textContent ?? "";
          for (const token of ["completed", "failed"]) {
            const start = text.indexOf(token);
            if (start < 0) continue;
            const range = document.createRange();
            range.setStart(textWalker.currentNode, start);
            range.setEnd(textWalker.currentNode, start + token.length);
            tokens.push({ token, rectCount: range.getClientRects().length });
          }
        }
        return tokens;
      });
    });
    assert.ok(diagnosticStatuses.length > 1);
    assert.ok(diagnosticStatuses.every((item) => item.rectCount === 1), "Diagnostic status words must stay intact at phone width");
  });

  workflow("SD20-queue-resume-cancel records resume and cancel outcomes for the selected jobs", async () => {
    await begin("queue");
    const runs = await $('//h3[normalize-space()="持久化工作流"]/ancestor::section[1]');
    await runs.waitForDisplayed();
    await openMenu(await runs.$("li*=章节分析"));
    await defer("resume_workflow_run");
    await click(await menuItem("继续运行"), "恢复第501次章节分析");
    const request = await pending("resume_workflow_run");
    assert.deepEqual(request.input, { workflow_run_id: 501 });
    const original = seed.invoke.list_workflow_runs[0];
    const resumed = {
      ...original, status: "running", can_resume: false, error_message: null,
      active_task_id: "ui-resumed-501", completed_at: null,
      updated_at: "2026-09-25T09:10:00Z",
    };
    const task = {
      id: resumed.active_task_id, task_type: "llm_call", status: "running", priority: "high",
      task_scope: "chapter", root_project_id: 1, source_snapshot_id: null,
      created_at: resumed.updated_at, started_at: resumed.updated_at, completed_at: null,
      progress: { current: 7, total: 8, message: "继续校验人物行动证据", meta: null },
    };
    const workflows = [...seed.invoke.list_active_workflows, {
      task_id: task.id, task_scope: task.task_scope, root_project_id: 1, source_snapshot_id: null,
      progress: task.progress, progress_events: [],
    }];
    const resumedRuns = seed.invoke.list_workflow_runs.map((item) => item.id === resumed.id ? resumed : item);
    const resumedTasks = [task, ...seed.invoke.get_task_history];
    const stats = { ...seed.invoke.get_queue_stats, llm_queue: { ...seed.invoke.get_queue_stats.llm_queue, running: 2 } };
    await respond("resume_workflow_run", resumed, {
      fixtures: { list_workflow_runs: resumedRuns, list_active_workflows: workflows, get_task_history: resumedTasks, get_queue_stats: stats },
    });
    await browser.waitUntil(async () => (await runs.$("li*=章节分析").getText()).includes("运行中"));
    await capture("resumed-workflow", { anchor: '//h3[normalize-space()="持久化工作流"]/ancestor::section[1]' });
    const active = await $('//h3[normalize-space()="活跃工作流"]/ancestor::section[1]');
    const video = await active.$('.//ul/li[.//span[normalize-space()="ui-task-video"]]');
    await video.waitForDisplayed();
    assert.match(await video.getText(), /视频任务/);
    await openMenu(video);
    await capture("active-cancel-menu");
    await defer("cancel_workflow");
    await click(await menuItem("取消工作流"), "取消选定的视频任务");
    const cancel = await pending("cancel_workflow");
    assert.deepEqual(cancel.input, { task_id: "ui-task-video", workflow_run_id: null });
    await respond("cancel_workflow", null, {
      fixtures: {
        list_active_workflows: workflows.filter((item) => item.task_id !== "ui-task-video"),
        get_queue_stats: { ...stats, video_queue: { ...stats.video_queue, running: 0 } },
        get_task_history: resumedTasks.map((item) => item.id === "ui-task-video" ? { ...item, status: "cancelled", completed_at: "2026-09-25T09:11:00Z" } : item),
        list_workflow_runs: resumedRuns.map((item) => item.id === 502 ? { ...item, status: "cancelled", active_task_id: null, completed_at: "2026-09-25T09:11:00Z" } : item),
      },
    });
    await browser.waitUntil(async () => (await active.$$(":scope > ul > li")).length === workflows.length - 1);
    assert.equal(await active.$('.//ul/li[.//span[normalize-space()="ui-task-video"]]').isExisting(), false);
    await capture("cancelled-active-job", { anchor: '//h3[normalize-space()="活跃工作流"]/ancestor::section[1]' });
  });

  workflow("SD21-account-profile expands and saves the current account profile", async () => {
    await begin("about");
    await openDetails("个人资料");
    const email = await inputFor("邮箱");
    const avatar = await inputFor("头像地址");
    await email.waitForDisplayed();
    assert.equal(await email.getAttribute("type"), "email");
    assert.equal(await avatar.getAttribute("type"), "url");
    await capture("profile-expanded", { anchor: '//summary[contains(normalize-space(),"个人资料")]' });
    await email.setValue("updated-ui-editor@example.invalid");
    await capture("profile-draft", { anchor: '//summary[contains(normalize-space(),"个人资料")]' });
    await defer("update_user_profile");
    await click(await button("保存资料"), "保存当前账户资料");
    const request = await pending("update_user_profile");
    assert.equal(request.input.email, "updated-ui-editor@example.invalid");
    const updated = { ...seed.invoke.get_current_user, ...request.input };
    await respond("update_user_profile", updated, { fixtures: { get_current_user: updated } });
    await browser.waitUntil(async () => !(await button("保存资料").isEnabled()));
    assert.equal(await email.getValue(), request.input.email);
    await openDetails("登录修复");
    await capture("session-repair-explanation", { anchor: '//summary[contains(normalize-space(),"登录修复")]' });
    step("凭证刷新未执行；无真实刷新凭证，不伪造TokenPair成功响应");
  });

  workflow("SD22-account-password submits with Enter and preserves fake input after failure", async () => {
    await begin("about");
    await openDetails("修改密码");
    const oldPassword = await inputFor("当前密码");
    const newPassword = await inputFor("新密码");
    assert.equal(await oldPassword.getAttribute("autocomplete"), "current-password");
    assert.equal(await newPassword.getAttribute("autocomplete"), "new-password");
    await oldPassword.setValue(fakePassword + "-old");
    await newPassword.setValue(fakePassword + "-new");
    await capture("password-filled", { anchor: '//summary[contains(normalize-space(),"修改密码")]' });
    await defer("change_user_password");
    await newPassword.click();
    await browser.keys("Enter");
    step("在新密码框按 Enter 提交");
    const request = await pending("change_user_password");
    assert.equal(request.input.old_password, fakePassword + "-old");
    assert.equal(request.input.new_password, fakePassword + "-new");
    await respond("change_user_password", {
      ...seed.variants.readFailure.body, message: "密码更新失败：模拟服务暂时不可用。",
    }, { status: 503 });
    await button("更新密码").waitForEnabled();
    assert.equal(await oldPassword.getValue(), fakePassword + "-old");
    assert.equal(await newPassword.getValue(), fakePassword + "-new");
    await capture("password-failure-retains-fields", { anchor: '//summary[contains(normalize-space(),"修改密码")]' });
    await oldPassword.setValue("");
    await newPassword.setValue("");
  });

  workflow("SD23-account-logout asks before losing an unsaved profile", async () => {
    await begin("about");
    await openDetails("个人资料");
    const email = await inputFor("邮箱");
    const saved = await email.getValue();
    await email.setValue("unsaved-ui-editor@example.invalid");
    await defer("revoke_session");
    const account = await section("当前账户");
    await openMenu(account);
    await click(await menuItem("退出登录"), "带资料草稿尝试退出登录");
    await dialog("未保存修改").waitForDisplayed({ timeout: 5000 });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "revoke_session")), false);
    await capture("logout-draft-confirmation");
    await click(await dialog("未保存修改").$('button=取消'), "取消退出，保留账户和草稿");
    await dialog("未保存修改").waitForExist({ reverse: true });
    assert.equal(await email.getValue(), "unsaved-ui-editor@example.invalid");
    await email.setValue(saved);
  });

  workflow("SD24-account-storage expands locations and confirms a simulated orphan cleanup", async () => {
    await begin("about");
    await openDetails("应用详情");
    await capture("application-details", { anchor: '//summary[contains(normalize-space(),"应用详情")]' });
    const paths = await section("存储位置");
    await click(await paths.$("summary"), "查看全部存储位置");
    await capture("storage-locations", { anchor: 'section[aria-label="存储位置"]' });
    await setFixtures({
      get_storage_paths: { ...seed.invoke.get_storage_paths, qdrant_url: "" },
    });
    await click(await paths.$('button[aria-label="刷新路径"]'), "刷新未配置 Qdrant 的存储路径");
    await browser.waitUntil(async () => (await paths.getText()).includes("未配置 A4N_QDRANT_URL"));
    const qdrantHasCopyButton = await browser.execute(() => {
      const label = [...document.querySelectorAll("span")]
        .find((element) => element.textContent?.trim() === "RAG 索引 (Qdrant URL)");
      return Boolean(label?.parentElement?.querySelector("button"));
    });
    assert.equal(qdrantHasCopyButton, false);
    const cleanup = await section("存储清理");
    await setFixtures({ list_orphan_project_data_dirs: seed.variants.orphanScan });
    await click(await cleanup.$("button=扫描"), "扫描独立的孤立目录测试场景");
    await browser.waitUntil(async () => (await cleanup.getText()).includes("1 个目录"));
    await click(await cleanup.$("summary"), "查看扫描结果目录");
    await capture("orphan-scan", { anchor: 'section[aria-label="存储清理"]' });
    await openMenu(cleanup);
    await click(await menuItem("清理孤儿项目目录"), "打开清理确认");
    await dialog("清理孤儿项目目录").waitForDisplayed();
    assert((await dialog("清理孤儿项目目录").getText()).includes("项目编号：1"));
    assert.equal(await cleanup.$('button[aria-haspopup="menu"]').isExisting(), false);
    await capture("orphan-cleanup-confirmation", {
      anchor: '//div[@role="dialog"][.//h3[normalize-space()="清理孤儿项目目录"]]',
    });
    await click(await dialog("清理孤儿项目目录").$('button=取消'), "先取消清理");
    await dialog("清理孤儿项目目录").waitForExist({ reverse: true });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "cleanup_orphan_project_data_dirs")), false);
    await defer("list_orphan_project_data_dirs");
    await click(await cleanup.$("button=扫描"), "重新扫描并等待结果");
    await pending("list_orphan_project_data_dirs");
    assert.equal(await cleanup.$('button[aria-haspopup="menu"]').isExisting(), false);
    await respond("list_orphan_project_data_dirs", {
      code: "unavailable",
      message: "模拟扫描失败，请重新扫描。",
      retryable: true,
      trace_id: "ui-orphan-scan-failure",
      details: {},
    }, { status: 503 });
    await browser.execute((command) => {
      const index = window.__uiAudit.deferred.indexOf(command);
      if (index >= 0) window.__uiAudit.deferred.splice(index, 1);
    }, "list_orphan_project_data_dirs");
    await cleanup.$('[role="alert"]').waitForDisplayed();
    assert.equal(await cleanup.$('button[aria-haspopup="menu"]').isExisting(), false);
    await capture("orphan-scan-failed", { anchor: 'section[aria-label="存储清理"]' });
    await defer("list_orphan_project_data_dirs");
    await click(await cleanup.$("button=扫描"), "重新扫描孤儿目录");
    await pending("list_orphan_project_data_dirs");
    assert.equal((await cleanup.getText()).includes("1 个目录"), false);
    assert.equal(await cleanup.$('button[aria-haspopup="menu"]').isExisting(), false);
    await respond("list_orphan_project_data_dirs", seed.variants.orphanScan);
    await browser.execute((command) => {
      const index = window.__uiAudit.deferred.indexOf(command);
      if (index >= 0) window.__uiAudit.deferred.splice(index, 1);
    }, "list_orphan_project_data_dirs");
    await browser.waitUntil(async () => (await cleanup.getText()).includes("1 个目录"));
    await cleanup.$('[role="alert"]').waitForExist({ reverse: true });
    await capture("orphan-rescan-recovered", { anchor: 'section[aria-label="存储清理"]' });
    await defer("cleanup_orphan_project_data_dirs");
    await openMenu(cleanup);
    await click(await menuItem("清理孤儿项目目录"), "重新请求清理");
    await dialog("清理孤儿项目目录").waitForDisplayed();
    await click(await dialog("清理孤儿项目目录").$('button=删除'), "确认清理模拟目录");
    const cleanupRequest = await pending("cleanup_orphan_project_data_dirs");
    assert.deepEqual(cleanupRequest.input.project_ids, [1]);
    await respond("cleanup_orphan_project_data_dirs", seed.variants.orphanScan, {
      fixtures: { list_orphan_project_data_dirs: { items: [], total_count: 0, total_size_bytes: 0 } },
    });
    await browser.waitUntil(async () => (await cleanup.getText()).includes("未发现可清理目录"));
    await capture("orphan-cleanup-result", { anchor: 'section[aria-label="存储清理"]' });
  });

  workflow("SD27-advanced-backup supports cancellation, error recovery, and confirmed import", async () => {
    await begin("advanced");
    const panel = await section("服务器配置备份");
    await capture("advanced-backup-default", { anchor: 'section[aria-label="服务器配置备份"]' });
    await browser.sendCommand("Browser.setDownloadBehavior", { behavior: "allow", downloadPath: output });
    const backupContents = JSON.stringify({ version: 1, scope: "server", settings: [{ name: "ui-fixture", api_key: "ui-only-secret" }] });
    const exportButton = await panel.$("button*=导出配置文件");
    const importButton = await panel.$("button*=导入配置文件");
    const fileInput = await panel.$('input[type="file"]');
    assert.equal(await exportButton.getAttribute("type"), "button");
    assert.equal(await importButton.getAttribute("type"), "button");
    assert.equal(await fileInput.getAttribute("aria-label"), "导入配置文件");
    assert.equal(await fileInput.getAttribute("tabindex"), "-1", "The hidden file input must not duplicate the visible keyboard action");
    await browser.execute(() => {
      if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    });
    let tabsToExport = 0;
    while (!await exportButton.isFocused() && tabsToExport < 80) {
      await browser.keys("Tab");
      tabsToExport += 1;
    }
    assert.equal(await exportButton.isFocused(), true, "Backup export must be reachable with Tab");
    await browser.keys("Tab");
    assert.equal(await importButton.isFocused(), true, "The visible import action must follow export without a hidden tab stop");
    step("键盘顺序连续到达导出和导入按钮，隐藏文件输入不占用 Tab 顺序", { tabsToExport });

    await defer("export_all_configs_json");
    await click(exportButton, "导出服务器配置备份");
    const exportRequest = await pending("export_all_configs_json");
    assert.deepEqual(exportRequest, {});
    assert.equal(await exportButton.isEnabled(), false, "Backup actions must remain disabled while exporting");
    await respond("export_all_configs_json", backupContents);
    await browser.waitUntil(async () => await exportButton.isEnabled());
    const exportDetails = await panel.$("section.a4n-flat-surface:first-of-type details");
    await exportDetails.waitForDisplayed();
    await click(await exportDetails.$("summary"), "查看最近导出的文件名");
    const exportedFileName = await exportDetails.$("div").getText();
    assert.match(exportedFileName, /^a4n-server-config-\d{8}T\d{6}\.json$/);
    await browser.waitUntil(async () => (await readdir(output)).includes(exportedFileName), { timeout: 5000 });
    const exportedFilePath = join(output, exportedFileName);
    assert.equal(await readFile(exportedFilePath, "utf8"), backupContents, "The browser download must contain the exported JSON");
    await capture("advanced-backup-exported", { anchor: 'section[aria-label="服务器配置备份"]' });

    const selectBackupFile = async (contents, name = "ui-backup.json", deferRead = false) => browser.execute((text, fileName, delayedRead) => {
      const panel = document.querySelector('section[aria-label="服务器配置备份"]');
      const input = panel?.querySelector('input[type="file"]');
      if (!(input instanceof HTMLInputElement)) throw new Error("Server backup file input not found");
      const file = new File([text], fileName, { type: "application/json" });
      if (delayedRead) Object.defineProperty(file, "text", {
        value: () => new Promise((resolve) => { window.__uiAudit.resolveBackupRead = resolve; }),
      });
      const transfer = new DataTransfer();
      transfer.items.add(file);
      input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }, contents, name, deferRead);
    const selectDownloadedBackup = async () => fileInput.setValue(await browser.uploadFile(exportedFilePath));

    await defer("import_all_configs");
    step("选择空配置文件，确认错误反馈且不调用导入");
    await selectBackupFile("  ", "empty-backup.json");
    await browser.waitUntil(async () => (await browser.execute(() => document.body.innerText)).includes("配置文件内容为空"));
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "import_all_configs")), false);
    await browser.waitUntil(async () => importButton.isEnabled());

    await selectBackupFile("{}", "ui-backup.json", true);
    await browser.waitUntil(async () => (await panel.getText()).includes("正在准备导入…"));
    await capture("advanced-backup-preparing", { anchor: 'section[aria-label="服务器配置备份"]' });
    await browser.execute(() => window.__uiAudit.resolveBackupRead("{}"));
    const confirmation = await dialog("导入配置");
    await confirmation.waitForDisplayed();
    assert((await confirmation.getText()).includes("服务器密钥"));
    assert.match(await confirmation.$("button=导入完整配置").getAttribute("class"), /!border-current/);
    await capture("advanced-backup-confirmation", { anchor: '//div[@role="dialog"][.//h3[normalize-space()="导入配置"]]' });
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "import_all_configs")), false);
    await closeDialog("导入配置");
    await browser.waitUntil(async () => importButton.isEnabled());
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "import_all_configs")), false,
      "Cancelling import confirmation must not send a request");

    step("读取备份并在确认框取消，不创建导入请求");
    await selectBackupFile("{}", "ui-backup.json", true);
    await browser.waitUntil(async () => (await panel.getText()).includes("正在准备导入…"));
    await browser.execute(() => window.__uiAudit.resolveBackupRead("{}"));
    const failedConfirmation = await dialog("导入配置");
    await failedConfirmation.waitForDisplayed();
    await click(await failedConfirmation.$("button=导入完整配置"), "确认并验证失败可恢复");
    const failedRequest = await pending("import_all_configs");
    assert.equal(failedRequest.input.settings_json, "{}");
    assert.equal(failedRequest.input.overwrite, false);
    const importFailure = {
      ...seed.variants.readFailure.body,
      message: "服务器暂时无法完成配置导入，请稍后重试。",
    };
    await respond("import_all_configs", importFailure, { status: 503 });
    await browser.waitUntil(async () => (await browser.execute(() => document.body.innerText)).includes(importFailure.message));
    await browser.waitUntil(async () => importButton.isEnabled());
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "import_all_configs").length), 1);
    await capture("advanced-backup-import-failed", { anchor: 'section[aria-label="服务器配置备份"]' });

    step("选择刚下载的真实文件，服务器拒绝导入后保留重试入口");
    await selectDownloadedBackup();
    const retryConfirmation = await dialog("导入配置");
    await retryConfirmation.waitForDisplayed();
    await click(await retryConfirmation.$("button=导入完整配置"), "重试并确认导入备份");
    const request = await pending("import_all_configs");
    assert.equal(request.input.settings_json, backupContents);
    assert.equal(request.input.overwrite, false);
    assert((await panel.getText()).includes("导入中…"));
    await capture("advanced-backup-importing", { anchor: 'section[aria-label="服务器配置备份"]' });
    await click(await $('nav').$("button=账户与应用"), "离开仍在执行导入的高级设置页");
    await click(await $('nav').$("button=高级配置"), "返回高级设置页");
    const remountedPanel = await section("服务器配置备份");
    await browser.waitUntil(async () => (await remountedPanel.getText()).includes("导入中…"));
    assert.equal(await remountedPanel.$("button*=导入配置文件").isEnabled(), false);
    await click(await remountedPanel.$("summary=导入选项"), "查看导入进行中的选项状态");
    assert.equal(await remountedPanel.$('[role="switch"]').isEnabled(), false);
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "import_all_configs").length), 2);
    await capture("advanced-backup-remount-importing", { anchor: 'section[aria-label="服务器配置备份"]' });
    await respond("import_all_configs", ["模拟设置已导入"]);
    await browser.waitUntil(async () => (await remountedPanel.getText()).includes("从 JSON 备份恢复配置"));
    await browser.waitUntil(async () => (await browser.execute(() => document.body.innerText)).includes("导入完成：模拟设置已导入"));
    await capture("advanced-backup-completed", { anchor: 'section[aria-label="服务器配置备份"]' });
  });

  workflow("SD25-auth-login-register keeps failed submissions recoverable", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/?ui-auth=1");
    await $(".a4n-auth-page").waitForDisplayed();
    step("进入未认证页面");
    await capture("login");
    await $('input[autocomplete="username"]').setValue("ui-login-author");
    await $('input[autocomplete="current-password"]').setValue(fakePassword);
    await defer("login_user");
    await click(await $('.a4n-auth-page form button[type="submit"]'), "提交模拟登录");
    const login = await pending("login_user");
    assert.equal(login.input.username, "ui-login-author");
    assert.equal(login.input.password, fakePassword);
    await respond("login_user", seed.variants.authFailure.body, { status: 401 });
    await $('.a4n-auth-page form button[type="submit"]').waitForEnabled();
    await capture("login-failure");
    assert.equal(await $('input[autocomplete="username"]').getValue(), "ui-login-author");
    await click(await button("注册"), "切换注册工作流");
    await $('input[autocomplete="email"]').waitForDisplayed();
    await $('input[autocomplete="username"]').setValue("ui-registration-author");
    await $('input[autocomplete="email"]').setValue("ui-registration-author@example.invalid");
    await $('input[autocomplete="one-time-code"]').setValue("UI-DETAILS-INVALID-INVITE");
    await $('input[autocomplete="new-password"]').setValue(fakePassword);
    await capture("registration-filled");
    await defer("register_user");
    await click(await button("创建账户"), "提交模拟注册");
    const registration = await pending("register_user");
    assert.equal(registration.input.username, "ui-registration-author");
    assert.equal(registration.input.invite_code, "UI-DETAILS-INVALID-INVITE");
    await respond("register_user", {
      code: "invalid_invite", message: "邀请码无效或已被使用，请确认后重试。",
      retryable: false, trace_id: "ui-registration-failure", details: {},
    }, { status: 400 });
    await button("创建账户").waitForEnabled();
    await capture("registration-failure");
    assert.equal(await $('input[autocomplete="email"]').getValue(), "ui-registration-author@example.invalid");
  });

  workflow("SD26-auth-oauth exposes provider choice, waiting and callback errors", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/?ui-auth=1");
    await click(await button("使用第三方账号登录"), "打开第三方登录");
    await $("#oauth-login-panel").waitForDisplayed();
    await choose(await $("#oauth-login-panel").$('button[role="combobox"]'), "Google");
    await capture("oauth-provider-choice");
    await defer("oauth2_authorize_url");
    await click(await button("使用 Google 继续"), "请求浏览器授权地址");
    const request = await pending("oauth2_authorize_url");
    assert.equal(request.input.provider, "google");
    assert.equal(typeof request.input.code_challenge, "string");
    await capture("oauth-waiting");
    await respond("oauth2_authorize_url", {
      ...seed.variants.readFailure.body, message: "第三方登录服务暂时不可用，请稍后重试。",
    }, { status: 503 });
    await button("使用 Google 继续").waitForEnabled();
    await capture("oauth-service-error");
    await click(await button("返回用户名登录"), "取消第三方登录并返回");
    await $('input[autocomplete="current-password"]').waitForDisplayed();
    await browser.url("/auth/callback?ui-auth=1&error=access_denied");
    step("进入不存在有效授权请求的回调");
    await $(".a4n-auth-page [role=alert]").waitForDisplayed();
    await capture("oauth-callback-error");
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "oauth2_callback")), false);
    await click(await button("返回用户名登录"), "从回调错误返回");
  });

  for (const user of seed.invoke.list_all_users.filter((item) => item.role === "user").slice(0, 1)) {
    workflow("SD27-account-role-user preserves the non-admin account boundary", async () => {
      await install({
        get_current_user: user,
        get_theme_preference: { ...seed.invoke.get_theme_preference, user_id: user.id },
        list_themes: seed.invoke.list_themes.filter((item) => item.user_id === null),
      });
      await begin("about");
      await section("当前账户").waitForDisplayed();
      step("切换明确的模拟账户身份", { userId: user.id, role: user.role });
      await capture("account-persona");
      assert.equal(await $('section[aria-label="存储清理"]').isExisting(), false);
      assert.equal(await $('[data-package-lifecycle]').isExisting(), false);
      await begin("admin");
      await browser.waitUntil(async () => (await $(".a4n-settings-content").getText()).includes("管理员"));
      await capture("admin-denied");
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("get_admin_overview")), false);
      await begin("queue");
      await capture("queue-denied");
      assert.equal(await browser.execute(() => window.__uiAudit.commands.includes("get_queue_stats")), false);
    });
  }
  report.planned = this.tests.map((test) => ({ id: test.title.split(" ")[0], title: test.title }));
});
