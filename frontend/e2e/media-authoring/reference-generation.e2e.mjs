import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { login, clickButton, waitForNoDialog, captureBrowserDiagnostics } from "../full-project/user-flow.mjs";
import { callBrowserApiCommand } from "../full-project/webApi.mjs";

const fixture = JSON.parse(readFileSync(process.env.A4N_MEDIA_AUTHORING_CLI_REPORT, "utf8"));
const projectId = fixture.ui_mutation_project_ids?.reference;
assert.ok(fixture.success && projectId, "Run the media-authoring CLI acceptance first");
const artifacts = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
const checks = [];
let complete = false;
const dialog = () => $('.a4n-modal-layer:not([data-modal-state="exiting"]) [role="dialog"]');
const generate = () => $('button[aria-label="为“断裂树根”生成并绑定参考图"]');

async function configureImage(model) {
  await browser.execute(({ config, model }) => {
    const key = "a4n:local-runtime-settings:v1";
    const settings = JSON.parse(localStorage.getItem(key));
    if (!settings) throw new Error("Missing browser runtime settings");
    settings.image = [{ ...config, id: 901, has_api_key: true, is_default: true, model }];
    settings.default_source.image = "local";
    settings.next_id = Math.max(settings.next_id, 902);
    localStorage.setItem(key, JSON.stringify(settings));
  }, { config: fixture.runtime_config_snapshot.image, model });
}

async function openReferences(width) {
  await browser.setWindowSize(width, 1000);
  await browser.url(`/comic-project/${projectId}`);
  await $('[role="tablist"]').waitForExist();
  const selector = await $('[role="combobox"][id$="-select"]');
  if (await selector.isDisplayed()) {
    await selector.click();
    await $('[role="listbox"]').waitForDisplayed();
    const options = await $('[role="listbox"]').$$('[role="option"]');
    for (const option of options) {
      if ((await option.getText()).includes("分镜与参考图")) {
        await option.click();
        break;
      }
    }
  } else {
    await $('//button[@role="tab" and contains(.,"分镜与参考图")]').click();
  }
  await generate().waitForDisplayed();
  await generate().scrollIntoView({ block: "center" });
}

async function capture(name, width) {
  if (name !== "generating") {
    await browser.waitUntil(async () => await browser.execute(() => document.querySelectorAll(".a4n-modal-layer").length === 0));
  }
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  await browser.saveScreenshot(`${artifacts}/reference-${name}-${width}.png`);
  checks.push(`${name} ${width}px: zero horizontal overflow`);
}

async function waitBound(count) {
  await browser.waitUntil(async () => (await $("body").getText()).includes(`已绑定 ${count}/2 张`), { timeout: 30_000 });
}

describe("Generate and bind discovered manga references against the real Host", () => {
  before(async () => {
    await login();
    await browser.url("/settings");
    await clickButton("LLM 服务");
    await clickButton("新增 LLM 配置");
    for (const [label, value] of [
      ["配置名称", "参考图隔离验收"], ["模型", "a4n-virtual-text"],
      ["接口地址", process.env.A4N_BROWSER_E2E_PROVIDER_URL], ["访问密钥", "a4n-e2e-provider-key"],
    ]) await $(`//label[contains(.,"${label}")]//input`).setValue(value);
    await clickButton(/创建|保存/);
    await configureImage("a4n-reference-image-slow");
  });
  after(() => writeFileSync(`${artifacts}/reference-generation-report.json`, JSON.stringify({ success: complete, checks }, null, 2)));

  it("generates a discovered requirement, preserves binding on refresh and enforces the limit", async () => {
    for (const width of [1440, 1024, 390]) {
      await openReferences(width);
      assert.ok((await $("body").getText()).includes("叙述者的回忆"));
      assert.ok(!(await $("body").getText()).includes("未具名主角的回忆"));
      assert.equal(await generate().isEnabled(), true);
      await capture("empty", width);
    }
    checks.push("Saved reference names are displayed literally without the former narrator alias replacement");
    await browser.setWindowSize(1440, 1000);
    await generate().execute((element) => element.focus());
    assert.equal(await generate().isFocused(), true);
    await browser.keys("Enter");
    await dialog().waitForDisplayed();
    await browser.waitUntil(async () => /正在生成参考图/.test(await dialog().getText()));
    assert.equal(await generate().isEnabled(), false);
    await capture("generating", 1440);
    await browser.waitUntil(async () => /任务已完成/.test(await dialog().getText()), { timeout: 60_000 });
    await clickButton("关闭", dialog());
    await waitForNoDialog();
    await waitBound(1);
    await browser.waitUntil(async () => await generate().execute((element) => {
      const images = [...element.closest("article").querySelectorAll("img")];
      return images.length > 0 && images.every((image) => image.complete && image.naturalWidth > 0);
    }));
    checks.push("Keyboard starts a real image task; duplicate actions are disabled while running; the completed image is bound");
    for (const width of [1440, 1024, 390]) {
      await openReferences(width);
      await waitBound(1);
      await capture("bound", width);
    }
    checks.push("Bound reference and thumbnail survive page reload at all three widths");

    await openReferences(1024);
    const referenceWorkspace = await callBrowserApiCommand("get_media_authoring_workspace", {
      input: { project_id: projectId, chapter_id: null },
    });
    const pagesBeforeReadback = await callBrowserApiCommand("list_manga_page_prompts", {
      input: { project_id: projectId, chapter_id: referenceWorkspace.chapters[0].source.source_chapter_id },
    });
    const pageBeforeReadback = pagesBeforeReadback.find((page) => page.page_number === 1);
    assert.ok(pageBeforeReadback);
    assert.deepEqual(await captureBrowserDiagnostics(`${artifacts}/before-reference-readback`), []);
    await browser.execute(() => {
      const original = window.fetch;
      window.__referenceRestoreFetch = () => { window.fetch = original; };
      let rejectReadback = true;
      window.fetch = async (input, init) => {
        const url = typeof input === "string" ? input : input.url ?? String(input);
        if (rejectReadback && url.includes("/commands/list_manga_page_prompts")) {
          rejectReadback = false;
          throw new Error("reference readback failure fixture");
        }
        return original.call(window, input, init);
      };
    });
    try {
      const remove = await $('button[aria-label="从断裂树根移除参考图"]');
      await remove.scrollIntoView({ block: "center" });
      await remove.click();
      await browser.waitUntil(async () => (await $("body").getText()).includes("产物已保存，但读取最新状态失败"));
      await capture("readback-error", 1024);
      const expected = await captureBrowserDiagnostics(`${artifacts}/expected-reference-readback`);
      assert.equal(expected.length, 1, JSON.stringify(expected));
      assert.equal(expected[0].source, "console-api");
      assert.ok(expected[0].message.includes("后端调用失败: list_manga_page_prompts"), JSON.stringify(expected));
    } finally {
      await browser.execute(() => { window.__referenceRestoreFetch?.(); delete window.__referenceRestoreFetch; });
    }
    const pagesAfterReadback = await callBrowserApiCommand("list_manga_page_prompts", {
      input: { project_id: projectId, chapter_id: pageBeforeReadback.chapter_id },
    });
    const pageAfterReadback = pagesAfterReadback.find((page) => page.id === pageBeforeReadback.id);
    assert.equal(pageAfterReadback.reference_discovery.references.length, 0);
    await callBrowserApiCommand("save_manga_pipeline_artifact", { input: {
      project_id: projectId, chapter_id: pageAfterReadback.chapter_id, page_number: 1,
      stage: "reference_discovery", payload: pageBeforeReadback.reference_discovery,
      expected_prompt_updated_at: pageAfterReadback.updated_at,
    } });
    checks.push("A real unbind with failed state readback reports that changes were saved instead of claiming successful refresh");

    await configureImage("a4n-reference-image-fail");
    await openReferences(1024);
    await generate().click();
    await dialog().waitForDisplayed();
    await browser.waitUntil(async () => /reference image fixture failure/.test(await dialog().getText()), { timeout: 60_000 });
    await clickButton("关闭", dialog());
    await waitForNoDialog();
    await $('[role="alert"]').waitForDisplayed();
    await waitBound(1);
    assert.equal(await generate().isEnabled(), true);
    await capture("error", 1024);
    checks.push("Provider failure is visible, preserves the prior binding and allows retry");

    await configureImage("a4n-reference-image-slow");
    await openReferences(390);
    await generate().click();
    await dialog().waitForDisplayed();
    await browser.waitUntil(async () => /任务已完成/.test(await dialog().getText()), { timeout: 60_000 });
    await clickButton("关闭", dialog());
    await waitForNoDialog();
    await waitBound(2);
    assert.equal(await generate().isEnabled(), false);
    await capture("limit", 390);
    await browser.refresh();
    await generate().waitForDisplayed();
    await waitBound(2);
    assert.equal(await generate().isEnabled(), false);
    checks.push("Two references disable further generation and remain bound after reload");
    const workspace = await callBrowserApiCommand("get_media_authoring_workspace", { input: { project_id: projectId, chapter_id: null } });
    assert.ok(workspace.chapters.length > 0);
    complete = true;
  });
});
