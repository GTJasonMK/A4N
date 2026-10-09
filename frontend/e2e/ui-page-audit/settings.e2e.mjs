import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";

const seed = JSON.parse(await readFile(new URL("./settings-fixtures.json", import.meta.url)));
const output = fileURLToPath(new URL("../../../.dev-runtime/ui-pages/settings/", import.meta.url));
const phase = process.env.A4N_UI_PAGE_PHASE ?? "current";
const report = [];

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
    const data = JSON.parse(JSON.stringify({ ...visualFixtures, ...seed.invoke, pageSessions: seed.rest.pageSessions }), (_key, value) => {
      if (typeof value !== "string") return value;
      return value.replace(/https:\/\/ui\.invalid\/image-(\d+)\.png/g, (_match, number) => `${browser.options.baseUrl}/e2e/ui-page-audit/media/image-${number}.svg`);
    });
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
});
