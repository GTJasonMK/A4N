import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";

const seed = JSON.parse(await readFile(new URL("./settings-fixtures.json", import.meta.url)));
const output = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/settings/", import.meta.url));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";
const report = [];

function createPageFixtures() {
  return JSON.parse(JSON.stringify({ ...visualFixtures, ...seed.invoke, pageSessions: seed.rest.pageSessions }), (_key, value) => {
    if (typeof value !== "string") return value;
    return value.replace(/https:\/\/ui\.invalid\/image-(\d+)\.png/g, (_match, number) => `${browser.options.baseUrl}/e2e/ui-page-audit/media/image-${number}.svg`);
  });
}

async function openPromptAssembly({ failures = [], deferred = [], extraCall = false, blockLimit = false } = {}) {
  await browser.setWindowSize(1440, 1000);
  const fixtures = {
    prompt_assembly_save: null,
    prompt_assembly_reset: null,
  };
  if (extraCall) {
    const base = seed.invoke.prompt_assembly_list[0];
    fixtures.prompt_assembly_list = [base, {
      ...base,
      call_key: "OtherService.custom_generation",
    }];
  }
  if (blockLimit) {
    const base = seed.invoke.prompt_assembly_list[0];
    fixtures.prompt_assembly_list = [{
      ...base,
      project_context_variables: ["project_info.title"],
    }];
    fixtures.prompt_assembly_get = {
      ...seed.invoke.prompt_assembly_get,
      config: {
        blocks: Array.from({ length: 64 }, (_, index) => ({
          id: `assembly-block-${index + 1}`,
          kind: "custom",
          value: `已确认模块 ${index + 1}`,
          enabled: true,
        })),
      },
      project_context: { title: "雾港来信" },
    };
  }
  const query = new URLSearchParams({
    "ui-run": String(Date.now()),
    "ui-fixtures": JSON.stringify(fixtures),
  });
  if (failures.length > 0) query.set("ui-fail", JSON.stringify(failures));
  for (const command of deferred) query.append("ui-defer", command);
  await browser.url(`/settings?${query}#prompt-assembly`);
  const region = await $(".a4n-settings-content");
  await region.waitForDisplayed();
  const selectedPrompt = await region.$("main h3");
  await browser.waitUntil(async () => (await selectedPrompt.getText()).includes("灵感对话"));
  return region;
}

async function openAdminUsers() {
  await browser.setWindowSize(1440, 1000);
  const query = new URLSearchParams({
    "ui-run": String(Date.now()),
    "ui-fixtures": JSON.stringify({}),
  });
  await browser.url(`/settings?${query}#admin`);
  const region = await $(".a4n-settings-content");
  await region.waitForDisplayed();
  await region.$("button=用户").click();
  return region;
}

async function openPromptTabWithFailures(failures = [], deferred = []) {
  await browser.setWindowSize(1440, 1000);
  const query = new URLSearchParams({
    "ui-run": String(Date.now()),
    "ui-fixtures": JSON.stringify({ prompt_load_defaults: 2 }),
  });
  if (failures.length > 0) query.set("ui-fail", JSON.stringify(failures));
  for (const command of deferred) query.append("ui-defer", command);
  await browser.url(`/settings?${query}#prompt`);
  const region = await $(".a4n-settings-content");
  await region.waitForDisplayed();
  return region;
}

async function openPageSessionWithFailures(failures) {
  const pageSessions = createPageFixtures().pageSessions;
  pageSessions.createResponse.session.status = "awaiting_attach";
  const query = new URLSearchParams({
    "ui-run": String(Date.now()),
    "ui-fixtures": JSON.stringify({
      get_current_user: { ...seed.invoke.get_current_user, page_cli_enabled: true },
      pageSessions,
    }),
    "ui-fail": JSON.stringify(failures),
  });
  await browser.url(`/settings?${query}#about`);
  const bar = await $('[aria-label="页面 CLI 会话"]');
  await bar.waitForDisplayed();
  return bar;
}

async function selectOtherAssemblyCall(main) {
  await main.$('button[role="combobox"][aria-label="模型调用"]').click();
  const listbox = await $('[role="listbox"]');
  await listbox.waitForDisplayed();
  const option = await $('//div[@role="option" and contains(normalize-space(.), "custom generation")]');
  await option.waitForDisplayed();
  await option.click();
}

async function waitForAssemblyPreview(content, previousReadCount) {
  await browser.waitUntil(() => browser.execute((expected) => {
    const requested = window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_get")
      .slice(expected.previousReadCount)
      .some((item) => item
      && item.body.input?.preview_config?.blocks.some((block) => block.value === expected.content));
    const updating = [...document.querySelectorAll("span")]
      .some((element) => element.textContent?.trim() === "更新中");
    return requested && !updating;
  }, { content, previousReadCount }), { timeout: 5000, timeoutMsg: "The edited assembly preview did not finish" });
}

const pages = [
  ["S01-llm", "llm"], ["S02-image", "image"], ["S03-video", "video"], ["S04-embedding", "embedding"],
  ["S05-web-search", "web-search"], ["S06-agent-skills", "agent"], ["S07-prompts", "prompt"],
  ["S08-themes", "theme"], ["S09-backup", "advanced"], ["S10-admin", "admin"],
  ["S17-queue", "queue"], ["S18-analytics", "logs"], ["S19-prompt-assembly", "prompt-assembly"],
  ["S20-account", "about"],
];

describe("Settings pages with populated records", () => {
  before(async () => {
    await mkdir(output, { recursive: true });
    const data = createPageFixtures();
    await installUiFixtures(browser, data);
    await browser.url("/");
    await $(".a4n-home-content").waitForDisplayed();
    await browser.execute(async (models) => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      for (const item of models.llm) await local.createLocalLlmConfig(item);
      for (const item of models.image) await local.createLocalImageConfig(item);
      for (const item of models.video) await local.createLocalVideoConfig(item);
      for (const item of models.embedding) await local.createLocalEmbeddingConfig(item);
      await local.updateLocalWebToolsConfig(models.webTools.search, models.webTools.fetch);
    }, seed.localModels);
  });

  after(async () => {
    await writeFile(`${output}/${phase}-${Date.now()}-report.json`, JSON.stringify({ phase, pages: pages.map(([id]) => id), captured: report }, null, 2));
  });

  afterEach(async function () {
    if (this.currentTest.state !== "failed") return;
    const id = this.currentTest.title.split(" ")[0];
    const state = await browser.execute(() => ({ url: location.href, text: document.body.innerText, errors: window.__uiAudit?.errors, unexpected: window.__uiAudit?.unexpected }));
    await browser.saveScreenshot(`${output}/${phase}-${id}-failure.png`);
    await writeFile(`${output}/${phase}-${id}-failure.json`, JSON.stringify(state, null, 2));
  });

  for (const [id, tab] of pages) {
    it(`${id} renders the populated workspace`, async () => {
      await browser.setWindowSize(1440, 1000);
      await browser.url("/");
      await browser.url(`/settings#${tab}`);
      await $(".a4n-settings-content").waitForDisplayed();
      await browser.pause(650);
      for (const width of [1440, 900, 390]) {
        await browser.setWindowSize(width, 1000);
        await browser.pause(300);
        const state = await browser.execute(() => ({
          width: innerWidth, height: innerHeight,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          text: document.querySelector(".a4n-settings-content").innerText,
          verticalScroll: [...document.querySelectorAll(".a4n-settings-content, .a4n-settings-content *")]
            .filter((element) => ["auto", "scroll"].includes(getComputedStyle(element).overflowY))
            .map((element) => ({
              tag: element.tagName.toLowerCase(),
              className: typeof element.className === "string" ? element.className : "",
              scrollHeight: element.scrollHeight,
              clientHeight: element.clientHeight,
              scrollTop: element.scrollTop,
            }))
            .filter((element) => element.scrollHeight > element.clientHeight + 1),
          unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors,
        }));
        const file = `${phase}-${id}-${width}.png`;
        await browser.saveScreenshot(`${output}/${file}`);
        report.push({ id, tab, file, ...state });
        assert.equal(state.width, width);
        assert.equal(state.overflow, 0);
        if (tab === "prompt-assembly") {
          assert(
            state.verticalScroll.some((element) => element.tag === "ul"),
            `The prompt list must scroll at ${width}px`,
          );
        }
        assert.deepEqual(state.unexpected, []);
        assert.deepEqual(state.errors, []);
        const clippedItems = await browser.execute(() => [...document.querySelectorAll(".a4n-settings-content aside li")]
          .filter((item) => item.getBoundingClientRect().right > item.closest("aside").getBoundingClientRect().right + 1)
          .map((item) => item.textContent));
        assert.deepEqual(clippedItems, [], "Configuration list cards must fit inside their rail");
      }
      if (tab === "prompt-assembly") {
        const target = await browser.execute(() => {
          const list = document.querySelector(".a4n-settings-content aside ul");
          if (!list) throw new Error("Prompt list is missing");
          const rect = list.getBoundingClientRect();
          list.scrollTop = 0;
          return {
            x: Math.round(rect.left + rect.width / 2),
            y: Math.round(rect.top + rect.height * 0.75),
            maxScroll: list.scrollHeight - list.clientHeight,
          };
        });
        assert(target.maxScroll > 0, "The prompt list must have vertical overflow on phone width");
        await browser.performActions([{
          type: "pointer",
          id: "prompt-list-touch-scroll",
          parameters: { pointerType: "touch" },
          actions: [
            { type: "pointerMove", duration: 0, x: target.x, y: target.y },
            { type: "pointerDown", button: 0 },
            { type: "pause", duration: 80 },
            { type: "pointerMove", duration: 250, x: target.x, y: target.y - 160 },
            { type: "pointerUp", button: 0 },
          ],
        }]);
        await browser.releaseActions();
        await browser.waitUntil(
          () => browser.execute(() => document.querySelector(".a4n-settings-content aside ul")?.scrollTop > 0),
          { timeout: 2000, timeoutMsg: "Touch swipe did not scroll the prompt list" },
        );
      }
    });
  }

  it("S19-prompt-assembly switches prompts without leaving the settings page", async () => {
    await browser.setWindowSize(900, 1000);
    await browser.url("/");
    await browser.url("/settings#prompt-assembly");
    const region = await $(".a4n-settings-content");
    await region.waitForDisplayed();
    const promptTitle = await region.$("main h3");
    await browser.waitUntil(async () => (await promptTitle.getText()).includes("灵感对话"));
    const selects = await region.$$('button[role="combobox"]');
    assert.equal(selects.length, 4, "Assembly controls must use the themed select component");
    const modelCall = await region.$('button[role="combobox"][aria-label="模型调用"]');
    await modelCall.click();
    await $('[role="listbox"]').waitForDisplayed();
    await browser.keys("Escape");
    await browser.waitUntil(() => browser.execute(() => document.querySelector('[role="listbox"]') == null));

    const search = await region.$('input[type="search"]');
    await search.setValue("蓝图");
    const blueprint = await $("//aside//button[contains(normalize-space(.), '蓝图生成')]");
    await blueprint.waitForDisplayed();
    await blueprint.click();
    await browser.waitUntil(async () => (await promptTitle.getText()).includes("蓝图生成"));

    const state = await browser.execute(() => ({
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      unexpected: window.__uiAudit.unexpected,
      errors: window.__uiAudit.errors,
    }));
    assert.equal(state.overflow, 0);
    assert.deepEqual(state.unexpected, []);
    assert.deepEqual(state.errors, []);
    await browser.saveScreenshot(`${output}/${phase}-S19-prompt-assembly-selection.png`);
  });

  it("UX-2313 saves theme appearance when the theme directory read fails", async () => {
    const query = new URLSearchParams({
      "ui-run": String(Date.now()),
      "ui-fixtures": JSON.stringify({
        get_theme_preference: { appearance: "light", user_id: 1 },
        update_theme_preference: { appearance: "dark", user_id: 1 },
      }),
      "ui-fail": JSON.stringify(Array.from({ length: 4 }, () => ({
        command: "list_themes",
        status: 503,
        message: "Injected theme directory failure",
      }))),
    });
    await browser.url(`/settings?${query}#theme`);
    const region = await $(".a4n-settings-content");
    await region.waitForDisplayed();
    const appearance = await region.$('[role="group"][aria-label="外观偏好"]');
    const currentAppearance = await browser.execute(async () =>
      (await import("/src/theme/activeTheme.ts")).getThemeAppearance(),
    );
    const nextAppearance = currentAppearance === "dark" ? "light" : "dark";
    const nextLabel = nextAppearance === "dark" ? "暗色" : "亮色";
    const nextOption = await appearance.$(`button=${nextLabel}`);
    await browser.execute((next) => {
      window.__uiAudit.fixtures.update_theme_preference.appearance = next;
    }, nextAppearance);
    await browser.waitUntil(async () => (await region.getText()).includes("Injected theme directory failure"));

    await nextOption.click();

    await browser.waitUntil(async () => (await nextOption.getAttribute("aria-pressed")) === "true");
    const requests = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "update_theme_preference"),
    );
    assert.equal(requests.at(-1)?.body.input.appearance, nextAppearance);
    assert((await region.getText()).includes("Injected theme directory failure"));
    report.push({ id: "UX-2313", appearanceSaved: nextAppearance, themeListErrorVisible: true });
  });

  it("UX-1710 retries the prompt assembly directory after a failed read", async () => {
    const region = await openPromptAssembly({
      failures: [
        { command: "prompt_assembly_list", status: 503, message: "Injected assembly directory failure" },
        { command: "prompt_assembly_list", status: 503, message: "Injected assembly directory failure" },
      ],
    });
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.getText()).includes("Injected assembly directory failure"));
    assert.equal((await main.getText()).includes("该提示词没有已注册的模型调用"), false);
    const error = await main.$('[role="alert"]');
    const readsBeforeRetry = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_list").length);
    await browser.execute(() => { window.__uiAudit.failNext = []; });
    await error.$("button").click();
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    const readsAfterRetry = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_list").length);
    assert.equal(readsAfterRetry, readsBeforeRetry + 1);
  });

  it("UX-1711 retries the configuration read without exposing an unknown draft", async () => {
    const region = await openPromptAssembly({
      failures: [
        { command: "prompt_assembly_get", status: 503, message: "Injected initial assembly read failure" },
        { command: "prompt_assembly_get", status: 503, message: "Injected initial assembly read failure" },
      ],
    });
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.getText()).includes("Injected initial assembly read failure"));
    assert.equal(await main.$('input[aria-label="启用模块"]').isExisting(), false);
    assert.equal(await main.$('button*=自定义模板').isExisting(), false);
    await browser.execute(() => { window.__uiAudit.failNext = []; });
    await main.$('[role="alert"] button').click();
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
  });

  it("UX-1712 clears the old call configuration when the new scope read fails", async () => {
    const region = await openPromptAssembly({ extraCall: true });
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "prompt_assembly_get", status: 503, message: "Injected alternate call read failure",
    }));
    await selectOtherAssemblyCall(main);
    await browser.waitUntil(async () => (await main.getText()).includes("Injected alternate call read failure"));
    assert.equal(await main.$('input[aria-label="启用模块"]').isExisting(), false);
    assert.equal(await main.$('button*=自定义模板').isExisting(), false);
    const request = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_get").at(-1));
    assert.equal(request.body.input.call_key, "OtherService.custom_generation");
    await main.$('[role="alert"] button').click();
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
  });

  it("UX-1713 hides the previous configuration while the selected call is loading", async () => {
    const region = await openPromptAssembly({ extraCall: true });
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    await browser.execute(() => window.__uiAudit.deferred.push("prompt_assembly_get"));
    await selectOtherAssemblyCall(main);
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "prompt_assembly_get")));
    assert.equal(await main.$('input[aria-label="启用模块"]').isExisting(), false);
    assert.equal(await main.$('button*=自定义模板').isExisting(), false);
    await browser.execute(() => {
      const index = window.__uiAudit.pending.findIndex((item) => item.command === "prompt_assembly_get");
      const [request] = window.__uiAudit.pending.splice(index, 1);
      request.resolve(Response.json(window.__uiAudit.fixtures.prompt_assembly_get));
    });
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
  });

  it("UX-1714 commits a successful save locally when its follow-up read fails", async () => {
    const region = await openPromptAssembly();
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    const addCustom = await main.$('button*=自定义模板');
    await addCustom.click();
    const custom = await main.$('textarea[aria-label="输入自定义 Tera 模板"]');
    await custom.waitForDisplayed();
    const previousReadCount = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_get").length);
    await custom.setValue("只使用已经确认的事实。");
    await waitForAssemblyPreview("只使用已经确认的事实。", previousReadCount);
    await browser.saveScreenshot(`${output}/${phase}-UX-1714-before-save.png`);
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "prompt_assembly_get", status: 503, message: "Injected save follow-up read failure",
    }));
    await main.$('//button[normalize-space(.)="保存"]').click();
    await browser.waitUntil(async () => (await main.getText()).includes("Injected save follow-up read failure"));
    assert.equal(await custom.getValue(), "只使用已经确认的事实。");
    assert.equal(await main.$('//button[normalize-space(.)="保存"]').isEnabled(), false);
    await browser.waitUntil(() => browser.execute(() => document.body.innerText.includes("调用装配已保存")));
    assert.equal((await main.getText()).includes("全局配置"), true);
    await browser.saveScreenshot(`${output}/${phase}-UX-1714-read-error.png`);
  });

  it("UX-1715 invalidates a reset configuration when the follow-up read fails", async () => {
    const region = await openPromptAssembly();
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "prompt_assembly_get", status: 503, message: "Injected reset follow-up read failure",
    }));
    await main.$('//button[normalize-space(.)="恢复此范围默认值"]').click();
    await browser.waitUntil(async () => (await main.getText()).includes("Injected reset follow-up read failure"));
    assert.equal(await main.$('input[aria-label="启用模块"]').isExisting(), false);
    await browser.waitUntil(() => browser.execute(() => document.body.innerText.includes("已移除该范围的自定义装配")));
    await main.$('[role="alert"] button').click();
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
  });

  it("UX-2151 releases assembly save before the background scope read completes", async () => {
    const region = await openPromptAssembly();
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    await main.$('button*=自定义模板').click();
    const custom = await main.$('textarea[aria-label="输入自定义 Tera 模板"]');
    await custom.setValue("保存后立即释放编辑器。");
    await browser.execute(() => window.__uiAudit.deferred.push("prompt_assembly_get"));
    await main.$('//button[normalize-space(.)="保存"]').click();
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "prompt_assembly_save")));
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "prompt_assembly_get")));
    await browser.waitUntil(async () => (await custom.isEnabled()) === true);
    assert.equal((await browser.$("body").getText()).includes("调用装配已保存"), true);
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const pending = audit.pending.find((item) => item.command === "prompt_assembly_get");
      pending.resolve(Response.json(audit.fixtures.prompt_assembly_get));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "prompt_assembly_get");
    });
  });

  it("UX-1716 explains save gates and starts custom modules empty", async () => {
    const region = await openPromptAssembly();
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    const enabled = await main.$('input[aria-label="启用模块"]');
    await enabled.click();
    await browser.waitUntil(async () => (await main.getText()).includes("至少启用一个模块才能保存"));
    assert.equal(await main.$('//button[normalize-space(.)="保存"]').isEnabled(), false);
    await enabled.click();
    await main.$('button[role="combobox"][aria-label="提示词模板"]').click();
    await $('//div[@role="option" and contains(normalize-space(.), "自定义模板")]').click();
    const custom = await main.$('textarea[aria-label="输入自定义 Tera 模板"]');
    await custom.waitForDisplayed();
    assert.equal(await custom.getValue(), "");
    assert.equal((await main.getText()).includes("每个模块都必须填写内容"), true);
    assert.equal(await main.$('//button[normalize-space(.)="保存"]').isEnabled(), false);
    await custom.setValue("已确认的自定义内容");
    await main.$('//button[contains(@class, "pyqt-btn-secondary") and contains(normalize-space(.), "自定义模板")]').click();
    await browser.waitUntil(async () => (await main.$$('textarea[aria-label="输入自定义 Tera 模板"]')).length === 2);
    const updatedCustomFields = await main.$$('textarea[aria-label="输入自定义 Tera 模板"]');
    assert.equal(await updatedCustomFields[1].getValue(), "");
    assert.equal(await main.$('//button[normalize-space(.)="保存"]').isEnabled(), false);
  });

  it("UX-1717 labels the assembly project selector as a project scope", async () => {
    const region = await openPromptAssembly();
    const scope = await region.$('button[role="combobox"][aria-label="项目范围"]');
    await browser.waitUntil(async () => await scope.isEnabled());
    await scope.click();
    await $('//div[@role="option" and contains(normalize-space(.), "雾港来信")]').click();
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.requests.some((item) =>
      item.command === "prompt_assembly_get" && item.body.input.project_id === 1)));
    const request = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_get").at(-1));
    assert.equal(request.body.input.project_id, 1);
  });

  it("UX-1718 explains that prompt selection is blocked while save is in flight", async () => {
    const region = await openPromptAssembly();
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.$('input[aria-label="启用模块"]').isExisting()) === true);
    await main.$('button*=自定义模板').click();
    const previousReadCount = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "prompt_assembly_get").length);
    await main.$('textarea[aria-label="输入自定义 Tera 模板"]').setValue("只使用已经确认的事实。");
    await waitForAssemblyPreview("只使用已经确认的事实。", previousReadCount);
    await browser.execute(() => window.__uiAudit.deferred.push("prompt_assembly_save"));
    await main.$('//button[normalize-space(.)="保存"]').click();
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "prompt_assembly_save")));
    await $("//aside//button[contains(normalize-space(.), '蓝图生成')]").click();
    await browser.waitUntil(() => browser.execute(() => document.body.innerText.includes("操作仍在进行")));
    assert.equal((await main.$("h3").getText()).includes("灵感对话"), true);
    await browser.execute(() => {
      const index = window.__uiAudit.pending.findIndex((item) => item.command === "prompt_assembly_save");
      const [request] = window.__uiAudit.pending.splice(index, 1);
      window.__uiAudit.fixtures.prompt_assembly_get.config = request.body.input.config;
      window.__uiAudit.fixtures.prompt_assembly_get.source = "global";
      request.resolve(Response.json(null));
    });
    await browser.waitUntil(() => browser.execute(() => document.body.innerText.includes("调用装配已保存")));
    assert.equal((await main.$("h3").getText()).includes("灵感对话"), true);
  });

  it("UX-2529 explains why project context cannot be added at the assembly limit", async () => {
    const region = await openPromptAssembly({ blockLimit: true });
    const main = await region.$("main");
    await browser.waitUntil(async () => (await main.getText()).includes("最多包含 64 个模块"));
    const scope = await main.$('button[role="combobox"][aria-label="项目范围"]');
    await browser.waitUntil(async () => scope.isEnabled());
    await scope.click();
    await $("//div[@role='option' and contains(normalize-space(.), '雾港来信')]").click();
    const contextField = await main.$('input[aria-label="项目标题"]');
    await contextField.waitForDisplayed();
    await contextField.click();
    await browser.waitUntil(async () => (await browser.$("body").getText()).includes("请先删除模块再添加"));
    assert.equal(await contextField.isSelected(), false);
    assert.equal(
      await main.$('.//button[contains(@class, "pyqt-btn-secondary") and normalize-space(.)="自定义模板"]').isEnabled(),
      false,
    );
    report.push({ id: "UX-2529", moduleLimitExplained: true, contextBlockUnchanged: true });
  });

  it("UX-2314 regression: create a config while its old list is unreadable", async () => {
    await browser.setWindowSize(1440, 1000);
    const query = new URLSearchParams({
      "ui-run": String(Date.now()),
      "ui-fixtures": JSON.stringify({}),
    });
    await browser.url(`/settings?${query}#llm`);
    const region = await $(".a4n-settings-content");
    await region.waitForDisplayed();
    await $(".a4n-config-editor").waitForDisplayed();
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "list_llm_config", status: 503, message: "Injected LLM config directory failure",
    }));
    await region.$('button[aria-label="刷新"]').click();
    await browser.waitUntil(async () => (await region.getText()).includes("Injected LLM config directory failure"));
    const startCreate = await region.$("button*=新增");
    await startCreate.waitForEnabled();
    await startCreate.click();
    const editor = await $(".a4n-config-editor");
    await editor.waitForDisplayed();
    await browser.pause(250);
    const createState = await browser.execute(() => ({
      editor: document.querySelector(".a4n-config-editor")?.innerText.slice(0, 100),
      dialogs: [...document.querySelectorAll('[role="dialog"]')].map((item) => item.innerText),
    }));
    assert((await editor.getText()).includes("新建 LLM 配置"), JSON.stringify(createState));
    await editor.$('//label[contains(normalize-space(.), "配置名称")]//input').setValue("新配置名称");
    await editor.$('//label[contains(normalize-space(.), "模型")]//input').setValue("deepseek-chat");
    await editor.$('//label[contains(normalize-space(.), "接口地址")]//input').setValue("https://api.deepseek.com/v1");
    const createButton = await editor.$("button=创建");
    assert.equal(await createButton.isEnabled(), true, await editor.getText());
    await createButton.click();
    await browser.waitUntil(async () => (await region.getText()).includes("新配置名称"));
    assert((await browser.$("body").getText()).includes("已创建"));
    assert((await region.getText()).includes("Injected LLM config directory failure"));
    report.push({ id: "UX-2314-regression", localConfigCreated: true, staleListErrorPreserved: true });
  });

  it("UX-2316 regression: create a user while the user list read is unavailable", async () => {
    const region = await openAdminUsers();
    await browser.waitUntil(async () => (await region.getText()).includes("林知微"));
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "list_all_users", status: 503, message: "Injected user directory failure",
    }));
    await region.$("button=更多").click();
    await $('[role="menuitem"]=刷新列表').click();
    await browser.waitUntil(async () => (await region.getText()).includes("Injected user directory failure"));
    await browser.execute(() => window.__uiAudit.deferred.push("create_user"));
    await region.$("button=新增用户").click();
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed();
    const fields = await dialog.$$("input");
    await fields[0].setValue("新增用户路径");
    await fields[1].setValue("new-user-path@example.invalid");
    await fields[2].setValue("UI-audit-only-Password-2026");
    await dialog.$("button=新增用户").click();
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "create_user")));
    const request = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "create_user").at(-1));
    assert.equal(request.body.input.username, "新增用户路径");
    const created = { ...seed.invoke.list_all_users[2], id: 99, username: request.body.input.username, email: request.body.input.email };
    await browser.execute((result) => {
      window.__uiAudit.fixtures.list_all_users = [...window.__uiAudit.fixtures.list_all_users, result];
      const pending = window.__uiAudit.pending.find((item) => item.command === "create_user");
      pending.resolve(Response.json(result));
    }, created);
    await browser.waitUntil(async () => (await region.getText()).includes("新增用户路径"));
    report.push({ id: "UX-2316-user-regression", createCommandSentWithListReadError: true });
  });

  it("UX-2316 regression: create an invite while its list read is unavailable", async () => {
    const region = await openAdminUsers();
    await region.$('summary*=邀请码').click();
    await browser.waitUntil(async () => (await region.getText()).includes("UI-DEMO-INVITE-UNUSED"));
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "list_invite_codes", status: 503, message: "Injected invite directory failure",
    }));
    await region.$("button=更多").click();
    await $('[role="menuitem"]=刷新列表').click();
    await browser.waitUntil(async () => (await region.getText()).includes("Injected invite directory failure"));
    await browser.execute(() => window.__uiAudit.deferred.push("create_invite_code"));
    await region.$("button=更多").click();
    await $('[role="menuitem"]*=新增邀请码').click();
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed();
    await dialog.$('input[aria-label="新增邀请码"]').setValue("UI-AUDIT-INVITE-READ-ERROR");
    await dialog.$("button=新增邀请码").click();
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "create_invite_code")));
    const request = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "create_invite_code").at(-1));
    assert.deepEqual(request.body.input, { code: "UI-AUDIT-INVITE-READ-ERROR" });
    const created = {
      id: 99,
      code: request.body.input.code,
      created_at: seed.fixtureDate,
      created_by_user_id: 1,
      used_at: null,
      used_by_user_id: null,
    };
    await browser.execute((result) => {
      window.__uiAudit.fixtures.list_invite_codes = [...window.__uiAudit.fixtures.list_invite_codes, result];
      const pending = window.__uiAudit.pending.find((item) => item.command === "create_invite_code");
      pending.resolve(Response.json(result));
    }, created);
    await browser.waitUntil(async () => (await region.getText()).includes("UI-AUDIT-INVITE-READ-ERROR"));
    report.push({ id: "UX-2316-invite-regression", createCommandSentWithListReadError: true });
  });

  it("UX-2315 regression: load default prompts while the prompt list is unreadable", async () => {
    const region = await openPromptTabWithFailures([], ["prompt_load_defaults"]);
    await region.$('textarea[aria-label="灵感对话"]').waitForDisplayed();
    await browser.execute(() => window.__uiAudit.failNext.push({
      command: "prompt_list", status: 503, message: "Injected prompt directory failure",
    }));
    await region.$('button[aria-label="刷新提示词"]').click();
    await browser.waitUntil(async () => (await region.getText()).includes("Injected prompt directory failure"));
    await region.$('button[aria-label="刷新提示词"]').waitForEnabled();
    const promptReadsBeforeDefaults = await browser.execute(() =>
      window.__uiAudit.requests.filter((item) => item.command === "prompt_list").length,
    );
    await region.$('button[aria-haspopup="menu"]').click();
    await $('[role="menuitem"]*=载入默认模板').click();
    const confirmation = await $('[role="dialog"]');
    await confirmation.waitForDisplayed();
    await confirmation.$("button=重新载入").click();
    await browser.waitUntil(() => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "prompt_load_defaults")));
    assert.equal(await browser.execute(() => window.__uiAudit.requests.some((item) => item.command === "prompt_load_defaults")), true);
    await browser.execute(() => {
      const pending = window.__uiAudit.pending.find((item) => item.command === "prompt_load_defaults");
      pending.resolve(Response.json(2));
    });
    await browser.waitUntil(() => browser.execute((count) =>
      window.__uiAudit.requests.filter((item) => item.command === "prompt_list").length > count,
    promptReadsBeforeDefaults));
    report.push({ id: "UX-2315-regression", loadDefaultsCommandSentWithListReadError: true });
  });

  it("UX-1726 recovers an initial Page authorization failure from the waiting session", async () => {
    const bar = await openPageSessionWithFailures([{
      command: "authorize", status: 503, message: "Injected Page authorization failure",
    }]);
    await browser.waitUntil(async () => (await bar.getText()).includes("Injected Page authorization failure"));
    assert.equal(await bar.getAttribute("data-page-session-status"), "awaiting_attach");
    const authorize = await bar.$("button=生成绑定码");
    await authorize.waitForEnabled();
    await authorize.click();
    await browser.waitUntil(async () => (await bar.getText()).includes("UI-PAGE-AUDIT-ONLY-0002"));
    const requests = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "authorize"));
    assert.equal(requests.length, 2);
  });

  it("UX-1731 stops Page context writes after the session goes offline", async () => {
    const pageSessions = createPageFixtures().pageSessions;
    pageSessions.createResponse.session.status = "awaiting_attach";
    pageSessions.events = [{
      kind: "offline",
      session: { ...pageSessions.createResponse.session, status: "offline", agent_id: null, operation_id: null },
      operation: null,
      route: null,
      reason: "heartbeat_expired",
    }];
    const query = new URLSearchParams({
      "ui-run": String(Date.now()),
      "ui-fixtures": JSON.stringify({
        get_current_user: { ...seed.invoke.get_current_user, page_cli_enabled: true },
        pageSessions,
      }),
    });
    await browser.url(`/settings?${query}#about`);
    const bar = await $('[aria-label="页面 CLI 会话"]');
    await browser.waitUntil(async () => (await bar.getAttribute("data-page-session-status")) === "offline");
    await browser.pause(200);
    const before = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "context").length);

    await browser.execute(() => { window.location.hash = "llm"; });
    await browser.pause(250);

    const after = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "context").length);
    assert.equal(after, before, "An offline Page Session must not receive another context write after navigation");
    assert.equal((await bar.getText()).includes("同步页面上下文失败"), false);
  });

  for (const [id, tab, price] of [
    ["S01-editor", "llm", "LLM 输入 / 百万 token"],
    ["S02-editor", "image", "图片 / 次"],
    ["S03-editor", "video", "视频 / 秒"],
    ["S04-editor", "embedding", "Embedding 输入 / 百万 token"],
  ]) it(`${id} exposes its own pricing and creation form on desktop and phone`, async () => {
    for (const width of [900, 390]) {
      await browser.setWindowSize(width, 1000);
      await browser.url("/");
      await browser.url(`/settings#${tab}`);
      await $('.a4n-settings-content aside button[aria-current="true"]').waitForDisplayed();
      if (width < 768) await $('.a4n-settings-content aside button[aria-current="true"]').click();
      const editor = await $(".a4n-config-editor");
      await editor.waitForDisplayed();
      const pricing = await editor.$('.//summary[contains(.,"模型价格")]');
      await pricing.scrollIntoView({ block: "center" });
      await pricing.click();
      assert((await editor.getText()).includes(price), "The selected capability price must be directly available");
      await editor.$('.//summary[normalize-space()="其他能力的价格"]').click();
      assert.equal((await editor.$$("input[inputmode=decimal]")).length, 7, "Other prices must remain editable");
      await browser.saveScreenshot(`${output}/${phase}-${id}-pricing-${width}.png`);
      report.push({ id, width, view: "pricing", primaryPrice: price });
      await browser.url("/");
      await browser.url(`/settings#${tab}`);
      const region = await $(".a4n-settings-content");
      await region.waitForDisplayed();
      const create = await region.$("button*=新增");
      await create.waitForDisplayed();
      await create.click();
      await $(".a4n-config-editor").waitForDisplayed();
      const fields = await $$(".a4n-config-editor input");
      assert.equal(await fields[0].getValue(), tab === "embedding" ? "基础本地哈希向量" : "", "Creation must start with its own default draft");
      await browser.saveScreenshot(`${output}/${phase}-${id}-create-${width}.png`);
      report.push({ id, width, view: "create", fields: fields.length });
    }
  });

  it("UX-1899 applies native config bounds and saves the LLM name on Enter", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#llm");
    const configName = seed.localModels.llm[0].name;
    const row = await $(`//aside//button[.//span[normalize-space()=${JSON.stringify(configName)}]]`);
    await row.waitForDisplayed();
    await row.click();

    const editor = await $(".a4n-config-editor");
    await editor.waitForDisplayed();
    const save = await editor.$('button[type="submit"]');
    const numericInputs = await editor.$$('input[type="number"]');
    const concurrency = numericInputs[0];
    const originalConcurrency = await concurrency.getValue();
    const nameInput = await editor.$('input:not([type]):not([inputmode])');
    const originalName = await nameInput.getValue();

    await concurrency.setValue("33");
    assert.equal(
      await browser.execute(() => document.querySelector('input[max="32"]')?.validity.rangeOverflow),
      true,
    );
    await save.click();
    const afterRejectedSave = await browser.execute(async (name) => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      return local.listLocalLlmConfigs().find((item) => item.name === name)?.max_concurrent;
    }, configName);
    assert.equal(afterRejectedSave, Number(originalConcurrency), "Invalid values must not reach the save handler");

    await concurrency.setValue(originalConcurrency);
    const outputValidity = await browser.execute(() => {
      const input = document.querySelectorAll('.a4n-config-editor input[type="number"]')[3];
      return { value: input.value, step: input.step, mismatch: input.validity.stepMismatch };
    });
    assert.deepEqual(outputValidity, { value: "8192", step: "1", mismatch: false });

    const enteredName = `${originalName} Enter`;
    await nameInput.setValue(enteredName);
    await browser.keys("Enter");
    await browser.waitUntil(() => browser.execute(async (name) => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      return local.listLocalLlmConfigs().some((item) => item.name === name);
    }, enteredName));

    await nameInput.setValue(originalName);
    await save.click();
    await browser.waitUntil(() => browser.execute(async (name) => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      return local.listLocalLlmConfigs().some((item) => item.name === name);
    }, originalName));
    await browser.saveScreenshot(`${output}/${phase}-UX-1899-config-native-submit.png`);
  });

  for (const [index, item] of seed.localModels.embedding.entries()) it(`S04-backend-${index} renders ${item.backend} and its actual model state`, async () => {
    for (const width of [900, 390]) {
      await browser.setWindowSize(width, 1000);
      await browser.url("/");
      await browser.url("/settings#embedding");
      const card = await $(`//aside//button[.//span[normalize-space()=${JSON.stringify(item.name)}]]`);
      await card.waitForDisplayed();
      await card.click();
      await $(".a4n-config-editor").waitForDisplayed();
      await browser.pause(200);
      await browser.saveScreenshot(`${output}/${phase}-S04-backend-${index}-${width}.png`);
      const text = await $(".a4n-config-editor").getText();
      const problems = await browser.execute(() => ({ unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors }));
      assert.deepEqual(problems, { unexpected: [], errors: [] });
      report.push({ id: `S04-backend-${index}`, width, backend: item.backend, text });
    }
  });

  for (const [id, tab] of [["S01", "llm"], ["S02", "image"], ["S03", "video"]]) {
    for (const width of [1440, 900, 390]) it(`${id}-discovery-${width} discovers, filters and recovers without replacing the selected model`, async () => {
      await browser.setWindowSize(width, 1000);
      await browser.url("/");
      await browser.url(`/settings#${tab}`);
      const selected = await $('.a4n-settings-content aside button[aria-current="true"]');
      await selected.waitForDisplayed();
      if (width < 768) await selected.click();
      const editor = await $(".a4n-config-editor");
      const input = await editor.$('input[role="combobox"]');
      await input.waitForDisplayed();
      const original = await input.getValue();
      const detect = await editor.$('button[title="检测模型列表"]');
      const command = `detect_${tab}_config_models`;
      const models = [
        { id: `ui-${tab}-draft`, name: "快速创作 · 草稿预览", owned_by: "UI Audit" },
        { id: `ui-${tab}-balanced`, name: "完整创作 · 长篇结构与多场景一致性核验专用模型", owned_by: "UI Audit" },
        { id: `ui-${tab}-final`, name: "最终交付 · 高质量输出", owned_by: "UI Audit" },
      ];
      await browser.execute((name) => window.__uiAudit.deferred.push(name), command);
      const resolve = async (body, status = 200) => {
        await browser.waitUntil(() => browser.execute((name) => window.__uiAudit.pending.some((item) => item.command === name), command));
        await browser.execute((name, value, code) => {
          const index = window.__uiAudit.pending.findIndex((item) => item.command === name);
          const [request] = window.__uiAudit.pending.splice(index, 1);
          request.resolve(Response.json(value, { status: code }));
        }, command, body, status);
      };
      const capture = async (stage) => {
        const state = await browser.execute(() => ({
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors,
          text: document.querySelector(".a4n-config-editor").innerText,
        }));
        const file = `${phase}-${id}-discovery-${width}-${stage}.png`;
        await browser.saveScreenshot(`${output}/${file}`);
        report.push({ id: `${id}-discovery`, width, stage, file, ...state });
        assert.equal(state.overflow, 0);
        assert.deepEqual(state.unexpected, []);
        assert.deepEqual(state.errors, []);
      };
      await detect.click();
      await browser.waitUntil(async () => !await detect.isEnabled());
      await capture("loading");
      await resolve({ models });
      await browser.waitUntil(async () => (await editor.getText()).includes("已加载 3 个模型"));
      assert.equal(await input.getValue(), original, "Discovery must preserve the selected model");
      await input.click();
      const list = await $('[role="listbox"]');
      await list.waitForDisplayed();
      await capture("catalog");
      const clipping = await list.execute((node) => [...node.querySelectorAll('[role="option"] span')].filter((item) => item.scrollWidth > item.clientWidth + 1).map((item) => item.textContent));
      assert.deepEqual(clipping, [], "Model names must remain readable in the picker");
      await browser.keys(["Control", "a"]);
      await browser.keys("balanced");
      await browser.waitUntil(async () => (await $$('[role="option"]')).length === 1);
      await browser.keys("Enter");
      assert.equal(await input.getValue(), models[1].id);
      await detect.click();
      await resolve({ code: "unavailable", message: "模型目录暂时不可用，请重试", retryable: true, trace_id: null, details: {} }, 503);
      await editor.$('[role="alert"]').waitForDisplayed();
      assert.ok((await editor.getText()).includes("模型目录暂时不可用，请重试"));
      assert.equal(await input.getValue(), models[1].id, "Failure must preserve the selected model");
      await capture("failure");
      await detect.click();
      await resolve({ models: [] });
      await browser.waitUntil(async () => (await editor.getText()).includes("未发现可用模型"));
      await input.click();
      await browser.keys(["Control", "a"]);
      await browser.keys(`ui-${tab}-manual`);
      assert.equal(await input.getValue(), `ui-${tab}-manual`);
      await capture("manual-entry");
    });
  }

  it("S02-size-edit keeps focus while entering a complete image dimension", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#image");
    const width = await $('input[aria-label="宽 1"]');
    await width.waitForDisplayed();
    await width.click();
    await browser.keys(["Control", "a"]);
    await browser.keys("1536");
    assert.equal(await $('input[aria-label="宽 1"]').getValue(), "1536");
    assert.equal(await $('input[aria-label="宽 1"]').isFocused(), true);
  });

  it("S05-web-tools saves search and fetch through the same keyboard form", async () => {
    await browser.setWindowSize(900, 1000);
    await browser.url("/");
    await browser.url("/settings#web-search");
    const searchTimeout = await $('//label[contains(.,"请求超时")]/following-sibling::input');
    await searchTimeout.waitForExist();
    await searchTimeout.setValue("75");
    const fetchTimeout = await $('input[form="web-tools-settings-form"][max="300"]');
    await fetchTimeout.scrollIntoView();
    await fetchTimeout.setValue("45");
    await fetchTimeout.click();
    await browser.keys("Enter");
    await browser.waitUntil(async () => browser.execute(() => !document.querySelector('[data-settings-dirty="true"]')));
    const saved = await browser.execute(async () => {
      const local = await import("/src/api/localRuntimeSettings.ts");
      return { search: local.getLocalWebSearchConfigSummary(), fetch: local.getLocalWebFetchConfig() };
    });
    assert.equal(saved.search.timeout_secs, 75);
    assert.equal(saved.fetch.timeout_secs, 45);
    await browser.saveScreenshot(`${output}/${phase}-S05-web-tools-saved-900.png`);
    await fetchTimeout.setValue("0");
    await browser.keys("Enter");
    assert.equal(await fetchTimeout.getAttribute("min"), "1");
    assert.equal(await browser.execute(() => document.querySelector('input[form="web-tools-settings-form"][max="300"]').validity.rangeUnderflow), true);
  });

  it("S17-load-card retains its populated content in a constrained flex layout", async () => {
    await browser.setWindowSize(900, 1000);
    await browser.url("/");
    await browser.url("/settings#queue");
    await $('h3=队列负载').waitForDisplayed();
    const bounds = await browser.execute(() => {
      const heading = [...document.querySelectorAll("h3")].find((node) => node.textContent === "\u961f\u5217\u8d1f\u8f7d");
      return heading.closest("section").getBoundingClientRect().height;
    });
    assert.ok(bounds > 150, `Statistics card collapsed to ${bounds}px`);
  });

  it("S06-revert returns to clean state when a skill is toggled back", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#agent");
    const checkbox = await $('input[type="checkbox"]');
    await checkbox.waitForDisplayed();
    await checkbox.click();
    await checkbox.click();
    assert.equal(await $('//button[normalize-space()="保存"]').isEnabled(), false);
    assert.equal(await browser.execute(() => document.querySelector('[data-settings-dirty="true"]') !== null), false);
  });

  it("S06-pending-save preserves edits made while the previous selection is saving", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#agent");
    await $('input[type="checkbox"]').waitForDisplayed();
    await browser.execute(() => { window.__uiAudit.deferred.push("update_agent_skill_configuration"); });
    const checks = await $$('input[type="checkbox"]');
    await checks[0].click();
    await $('//button[normalize-space()="保存"]').click();
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.pending.length > 0));
    await checks[1].click();
    const expected = await checks[1].isSelected();
    await browser.execute(() => {
      const pending = window.__uiAudit.pending[0];
      const input = pending.body.input;
      const profile = window.__uiAudit.fixtures.list_agent_skills.profiles.find((item) => item.capability === input.capability);
      pending.resolve(Response.json({ ...profile, skills: profile.skills.map((skill) => ({ ...skill, enabled: input.enabled_skill_ids.includes(skill.id) })) }));
    });
    await $('//button[normalize-space()="保存"]').waitForDisplayed();
    await browser.pause(150);
    assert.equal(await $$('input[type="checkbox"]')[1].isSelected(), expected);
    assert.equal(await browser.execute(() => document.querySelector('[data-settings-dirty="true"]') !== null), true);
  });

  it("S06-retry asks before replacing unsaved skill changes", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#agent");
    const checkbox = await $('input[type="checkbox"]');
    await checkbox.waitForDisplayed();
    const before = await checkbox.isSelected();
    await browser.execute(() => { window.__uiAudit.deferred.push("list_agent_skills"); });
    await $('[aria-label="刷新技能配置"]').click();
    await browser.waitUntil(async () => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "list_agent_skills")));
    await browser.execute(() => {
      const pending = window.__uiAudit.pending.find((item) => item.command === "list_agent_skills");
      pending.resolve(Response.json({ message: "fixture read failure" }, { status: 503 }));
    });
    const retry = await $('//button[normalize-space()="重试"]');
    await retry.waitForDisplayed();
    await checkbox.click();
    const edited = await checkbox.isSelected();
    assert.notEqual(edited, before);
    await retry.click();
    const discardDialog = await $('[role="dialog"]');
    await discardDialog.waitForDisplayed();
    await discardDialog.$('//button[normalize-space()="取消"]').click();
    assert.equal(await $('input[type="checkbox"]').isSelected(), edited);
    assert.equal(await $('//button[normalize-space()="保存"]').isEnabled(), true);
  });

  it("S06-refresh confirms before replacing unsaved skill changes", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#agent");
    const checkbox = await $('input[type="checkbox"]');
    await checkbox.waitForDisplayed();
    await checkbox.click();
    const edited = await checkbox.isSelected();
    const refresh = await $('[aria-label="刷新技能配置"]');
    assert.equal(await refresh.isEnabled(), true);
    await refresh.click();
    const cancelled = await $('[role="dialog"]');
    await cancelled.waitForDisplayed();
    await cancelled.$('//button[normalize-space()="取消"]').click();
    assert.equal(await checkbox.isSelected(), edited);
    assert.equal(await browser.execute(() => document.querySelector('[data-settings-dirty="true"]') !== null), true);

    await browser.execute(() => { window.__uiAudit.deferred.push("list_agent_skills"); });
    await refresh.click();
    const confirmation = await $('[role="dialog"]');
    await confirmation.waitForDisplayed();
    await confirmation.$('//button[normalize-space()="放弃草稿"]').click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "list_agent_skills"),
    ));
    await browser.execute(() => {
      const audit = window.__uiAudit;
      const pending = audit.pending.find((item) => item.command === "list_agent_skills");
      pending.resolve(Response.json(audit.fixtures.list_agent_skills));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "list_agent_skills");
    });
    await browser.waitUntil(async () => browser.execute(() =>
      document.querySelector('[data-settings-dirty="true"]') === null,
    ));
    assert.notEqual(await checkbox.isSelected(), edited);
  });

  it("B196-config-operation keeps a clean settings page open while its default change is pending", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/");
    await browser.url("/settings#llm");
    const inactive = seed.invoke.list_llm_config.find((item) => !item.is_default);
    const card = await $(`//aside//button[contains(normalize-space(.), ${JSON.stringify(inactive.name)})]`);
    await card.waitForDisplayed();
    await card.click();
    await $(".a4n-config-editor").waitForDisplayed();

    await browser.execute(() => { window.__uiAudit.deferred.push("set_default_llm_config"); });
    await $('.a4n-config-editor button*=设为默认').click();
    await browser.waitUntil(async () => browser.execute(() =>
      window.__uiAudit.pending.some((item) => item.command === "set_default_llm_config"),
    ));
    await $('nav[aria-label="设置分类"] button*=生图模型').click();

    const blocker = await $('[role="dialog"]');
    await blocker.waitForDisplayed();
    assert((await blocker.getText()).includes("操作仍在进行"));
    assert.equal(await blocker.$('button*=放弃草稿').isEnabled(), false);
    assert.equal(new URL(await browser.getUrl()).hash, "#llm");

    await browser.execute(() => {
      const audit = window.__uiAudit;
      const pending = audit.pending.find((item) => item.command === "set_default_llm_config");
      pending.resolve(Response.json(null));
      audit.pending = audit.pending.filter((item) => item !== pending);
      audit.deferred = audit.deferred.filter((command) => command !== "set_default_llm_config");
    });
    await browser.waitUntil(async () => new URL(await browser.getUrl()).hash === "#image");
    assert.equal(await blocker.isExisting(), false);
  });
});
