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
  ["S17-queue", "queue"], ["S18-analytics", "logs"], ["S20-account", "about"],
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
          unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors,
        }));
        const file = `${phase}-${id}-${width}.png`;
        await browser.saveScreenshot(`${output}/${file}`);
        report.push({ id, tab, file, ...state });
        assert.equal(state.width, width);
        assert.equal(state.overflow, 0);
        assert.deepEqual(state.unexpected, []);
        assert.deepEqual(state.errors, []);
        const clippedItems = await browser.execute(() => [...document.querySelectorAll(".a4n-settings-content aside li")]
          .filter((item) => item.getBoundingClientRect().right > item.closest("aside").getBoundingClientRect().right + 1)
          .map((item) => item.textContent));
        assert.deepEqual(clippedItems, [], "Configuration list cards must fit inside their rail");
      }
    });
  }

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
});
