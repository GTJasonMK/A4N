import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { fileURLToPath } from "node:url";
import { login, captureBrowserDiagnostics } from "../full-project/user-flow.mjs";
import { callBrowserApiCommand, requestBrowserApiCommand, waitForBrowserGenerationTask } from "../full-project/webApi.mjs";

const report = JSON.parse(readFileSync(process.env.A4N_MEDIA_AUTHORING_CLI_REPORT, "utf8"));
const ids = report.ui_project_ids;
const mutationIds = report.ui_mutation_project_ids;
const runtimeConfigSnapshot = report.runtime_config_snapshot;
const artifacts = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
assert.ok(report.success && ids && mutationIds && runtimeConfigSnapshot, "Run the CLI acceptance before checking its projects");

async function expectRejectedCommand(command, args, statuses, message) {
  assert.deepEqual(await captureBrowserDiagnostics(`${artifacts}/before-${command}`), []);
  const response = await requestBrowserApiCommand(command, args);
  assert.ok(statuses.includes(response.status), JSON.stringify(response));
  assert.match(JSON.stringify(response.body), message);
  const errors = await captureBrowserDiagnostics(`${artifacts}/expected-${command}`);
  assert.ok(errors.every((error) =>
    (statuses.includes(error.status) && error.url.endsWith(`/api/commands/${command}`))
    || (error.source === "network" && error.message.includes(`/api/commands/${command}`)
      && error.message.includes(String(response.status)))), JSON.stringify(errors));
}

describe("Media authoring against the real Host", () => {
  before(login);

  for (const [kind, count] of [["comic", 6], ["galgame", 7], ["video", 7]]) {
    it(`opens every ${kind} stage with saved artifacts on desktop and narrow screens`, async () => {
      console.info(`Opening ${kind} workspace`);
      await browser.setWindowSize(1440, 1000);
      await browser.url(`/${kind}-project/${ids[kind]}`);
      await $('[role="tablist"]').waitForDisplayed();
      console.info(`${kind} navigation loaded`);
      const tabs = await $('[role="tablist"]').$$('[role="tab"]');
      assert.equal(tabs.length, count);
      assert.ok((await tabs[0].getText()).includes("基础素材"));
      assert.equal(await tabs[0].getAttribute("aria-selected"), "true");
      for (const width of [1440, 1024, 390]) {
        await browser.setWindowSize(width, 1000);
        for (let index = 0; index < count; index += 1) {
          console.info(`${kind} stage ${index + 1} at ${width}px`);
          const tab = await $('[role="tablist"]').$$('[role="tab"]')[index];
          const selector = await $('[role="combobox"][id$="-select"]');
          if (await selector.isDisplayed()) {
            await selector.execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
            await selector.click();
            await $('[role="listbox"]').waitForDisplayed();
            const options = await $('[role="listbox"]').$$('[role="option"]');
            assert.equal(options.length, count);
            if (index === 0) await browser.saveScreenshot(`${artifacts}/real-${kind}-stage-menu-${width}.png`);
            await options[index].click();
          } else {
            await tab.execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
            await tab.click();
          }
          await browser.waitUntil(async () => await tab.getAttribute("aria-selected") === "true");
          const panelId = await tab.getAttribute("aria-controls");
          const panel = await $(`#${panelId}`);
          assert.equal(await panel.getAttribute("role"), "tabpanel");
          await browser.waitUntil(async () => await panel.execute((element) => !element.querySelector(".animate-spin")), { timeoutMsg: `${kind} stage ${index + 1} did not finish loading` });
          if (kind === "comic" && index === 4) {
            const headings = await panel.execute((element) => [...element.querySelectorAll("h3")]
              .filter((heading) => heading.textContent.trim() === "单页分镜").length);
            assert.equal(headings, 1, "分镜内容只应显示一个阶段标题");
          }
          const errors = await browser.execute(() => [...document.querySelectorAll('[role="alert"]')]
            .map((element) => element.textContent).filter((text) => /失败|错误/.test(text ?? "")));
          assert.deepEqual(errors, []);
          await browser.waitUntil(async () => await browser.execute(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth));
          await browser.saveScreenshot(`${artifacts}/real-${kind}-stage-${index + 1}-${width}.png`);
          if (index === 0) {
            await $('//button[normalize-space()="场景与道具"]').click();
            await browser.waitUntil(async () => await panel.execute((element) => !element.querySelector(".animate-spin")));
            await browser.saveScreenshot(`${artifacts}/real-${kind}-base-scenes-${width}.png`);
            await $('//button[normalize-space()="人物"]').click();
          }
        }
      }
      await browser.setWindowSize(1440, 1000);
      await $('[role="tablist"]').$$('[role="tab"]')[0].execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
      await $('[role="tablist"]').$$('[role="tab"]')[0].click();
      await $('[role="tab"][aria-selected="true"]').execute((element) => element.focus());
      await browser.keys("ArrowRight");
      await browser.waitUntil(async () => await $('[role="tablist"]').$$('[role="tab"]')[1].getAttribute("aria-selected") === "true");
      await browser.refresh();
      await browser.waitUntil(async () => await $('[role="tablist"]').$$('[role="tab"]')[1].getAttribute("aria-selected") === "true");
    });
  }

  it("saves and adopts a real comic plan while cancelling navigation preserves unsaved text", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url(`/comic-project/${mutationIds.draft}`);
    await $('[role="tablist"]').waitForDisplayed();
    await $('[role="tab"][id$="-tab-adaptation"]').click();
    const enterEdit = () => $('//button[normalize-space()="进入编辑"]');
    await enterEdit().waitForEnabled();
    await enterEdit().click();
    const objective = () => $('//label[span[normalize-space()="人物目标"]]/textarea');
    await objective().waitForDisplayed();
    const edited = `真实界面保存：保留怀表的发现与人物反应 ${Date.now()}`;
    await objective().setValue(edited);
    await $('[role="tab"][aria-selected="true"]').execute((element) => element.focus());
    await browser.keys("ArrowRight");
    await $('[role="dialog"]').waitForDisplayed();
    await browser.waitUntil(async () => await browser.execute(() => document.querySelector('[role="dialog"]')?.contains(document.activeElement)));
    await browser.keys("Tab");
    assert.ok(await browser.execute(() => document.querySelector('[role="dialog"]')?.contains(document.activeElement)));
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForDisplayed({ reverse: true });
    assert.equal(await objective().getValue(), edited);
    await browser.setWindowSize(390, 1000);
    const stageSelector = await $('[role="combobox"][id$="-select"]');
    await stageSelector.execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
    await stageSelector.click();
    await $('[role="listbox"]').waitForDisplayed();
    await $('[role="listbox"]').$$('[role="option"]')[3].click();
    await $('[role="dialog"]').waitForDisplayed();
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForDisplayed({ reverse: true });
    assert.equal(await objective().getValue(), edited);
    const origin = await $('[role="combobox"][aria-label="内容来源"]');
    await origin.execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
    await origin.click();
    await $('[role="listbox"]').waitForDisplayed();
    assert.equal((await $('[role="listbox"]').$$('[role="option"]')).length, 4);
    await browser.saveScreenshot(`${artifacts}/real-comic-origin-menu.png`);
    await browser.keys("Escape");
    await browser.setWindowSize(1440, 1000);
    await $('//button[normalize-space()="保存"]').click();
    await enterEdit().waitForDisplayed();
    const saved = await callBrowserApiCommand("get_media_authoring_workspace", { input: { project_id: mutationIds.draft, chapter_id: null } });
    assert.ok(saved.documents.some(({ record }) => record.status === "adopted" && record.document.units?.[0]?.objective === edited));
    await browser.refresh();
    await enterEdit().waitForEnabled();
    await enterEdit().click();
    await objective().waitForDisplayed();
    assert.equal(await objective().getValue(), edited);
    await $('//button[normalize-space()="取消编辑"]').click();
    await browser.saveScreenshot(`${artifacts}/real-comic-adopted.png`);
    await $('[role="tab"][id$="-tab-page_planning"]').click();
    await browser.waitUntil(async () => await $('[role="tab"][id$="-tab-page_planning"]').getAttribute("aria-selected") === "true");
  });

  it("uses one route guard to cancel or discard a plan draft", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url(`/comic-project/${mutationIds.draft}`);
    await $('[role="tab"][id$="-tab-adaptation"]').click();
    await $('//button[normalize-space()="进入编辑"]').waitForEnabled();
    await $('//button[normalize-space()="进入编辑"]').click();
    const objective = () => $('//label[span[normalize-space()="人物目标"]]/textarea');
    await objective().waitForDisplayed();
    const saved = await objective().getValue();
    await objective().setValue("未保存的返回导航回归");
    await $('button[aria-label="返回上一页"]').click();
    await $('[role="dialog"]').waitForDisplayed();
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForDisplayed({ reverse: true });
    assert.equal(await objective().getValue(), "未保存的返回导航回归");
    await $('button[aria-label="返回上一页"]').click();
    await $('[role="dialog"]').waitForDisplayed();
    await $('//button[normalize-space()="放弃草稿"]').click();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/");
    assert.equal(await $('[role="dialog"]').isExisting(), false);
    await browser.url(`/comic-project/${mutationIds.draft}`);
    await $('//button[normalize-space()="进入编辑"]').waitForEnabled();
    await $('//button[normalize-space()="进入编辑"]').click();
    await objective().waitForDisplayed();
    assert.equal(await objective().getValue(), saved);
  });

  it("keeps two real tabs consistent when one deletes and regenerates a comic image", async () => {
    const projectId = mutationIds.image;
    const firstTab = await browser.getWindowHandle();
    const address = new URL(`/comic-project/${projectId}`, browser.options.baseUrl).href;
    await browser.setWindowSize(1440, 1000);
    await browser.url(address);
    await $('[role="tablist"]').waitForDisplayed();
    const workspace = await callBrowserApiCommand("get_media_authoring_workspace", {
      input: { project_id: projectId, chapter_id: null },
    });
    const chapterId = workspace.artifacts.find((artifact) => artifact.kind === "manga_pages").source_chapter_id;
    assert.ok(chapterId > 0);
    const pages = () => callBrowserApiCommand("list_manga_page_prompts", {
      input: { project_id: projectId, chapter_id: chapterId },
    });
    const previous = (await pages()).find((page) => page.page_number === 1);
    assert.ok(previous.generated_image_id);
    const deletionInput = { project_id: projectId, chapter_id: chapterId, stage: "image", page_number: 1 };
    const preview = await callBrowserApiCommand("preview_delete_manga_pipeline_artifact", { input: deletionInput });
    const deletion = { input: { ...deletionInput, expected_impact_fingerprint: preview.fingerprint } };
    await browser.newWindow(address);
    const secondTab = await browser.getWindowHandle();
    try {
      await $('[role="tablist"]').waitForDisplayed();
      assert.equal((await pages()).find((page) => page.page_number === 1).generated_image_id, previous.generated_image_id);
      await browser.switchToWindow(firstTab);
      await callBrowserApiCommand("delete_manga_pipeline_artifact", deletion);
      await browser.switchToWindow(secondTab);
      await expectRejectedCommand("delete_manga_pipeline_artifact", deletion, [400, 404, 409], /不存在|已变化|刷新|已删除/);

      await browser.switchToWindow(firstTab);
      const generation = await callBrowserApiCommand("start_manga_pipeline", { runtime_config_snapshot: runtimeConfigSnapshot, input: {
        meta: { start_request_id: randomUUID() }, project_id: projectId,
        chapter_number: 1, page_number: 1, run_stage: "image", stop_after: "image",
        image_size: { width: 1024, height: 1024 },
      } });
      const completed = await waitForBrowserGenerationTask(generation);
      assert.equal(completed.handle.status, "completed", JSON.stringify(completed));
      let regenerated;
      await browser.waitUntil(async () => {
        regenerated = (await pages()).find((page) => page.page_number === 1);
        return regenerated.generated_image_id != null && regenerated.generated_image_id !== previous.generated_image_id;
      }, { timeout: 120_000, interval: 250, timeoutMsg: "删除后没有产生新的漫画成图" });
      await browser.switchToWindow(secondTab);
      await browser.refresh();
      await $('[role="tablist"]').waitForDisplayed();
      assert.equal((await pages()).find((page) => page.page_number === 1).generated_image_id, regenerated.generated_image_id);
      const duplicate = await callBrowserApiCommand("start_manga_pipeline", { runtime_config_snapshot: runtimeConfigSnapshot, input: {
        meta: { start_request_id: randomUUID() }, project_id: projectId,
        chapter_number: 1, page_number: 1, run_stage: "image", stop_after: "image",
        image_size: { width: 1024, height: 1024 },
      } });
      const rejected = await waitForBrowserGenerationTask(duplicate);
      assert.equal(rejected.handle.status, "failed", JSON.stringify(rejected));
      assert.match(JSON.stringify(rejected.terminal_event), /请先删除/);
      assert.equal((await pages()).find((page) => page.page_number === 1).generated_image_id, regenerated.generated_image_id);
      await browser.saveScreenshot(`${artifacts}/real-comic-two-tabs.png`);
    } finally {
      await browser.switchToWindow(secondTab);
      await browser.closeWindow();
      await browser.switchToWindow(firstTab);
    }
  });

  it("persists uploaded user references separately from system references", async () => {
    const projectId = mutationIds.image;
    await browser.setWindowSize(1440, 1000);
    await browser.url(`/comic-project/${projectId}`);
    await $('[role="tablist"]').waitForDisplayed();
    await $('[role="tablist"]').$$('[role="tab"]')[4].click();
    const workspace = await callBrowserApiCommand("get_media_authoring_workspace", {
      input: { project_id: projectId, chapter_id: null },
    });
    const chapterId = workspace.artifacts.find((artifact) => artifact.kind === "manga_pages").source_chapter_id;
    const readPage = async () => (await callBrowserApiCommand("list_manga_page_prompts", {
      input: { project_id: projectId, chapter_id: chapterId },
    })).find((page) => page.page_number === 1);
    const original = await readPage();
    const originalBinding = original.reference_bindings.find((binding) => binding.bound_asset?.asset_kind === "character_portrait");
    assert.ok(originalBinding, "The real fixture must include a system portrait binding");
    assert.deepEqual(originalBinding.user_added_assets, []);
    const characterName = originalBinding.character_name;
    const uploadSelector = `input[type="file"][aria-label=${JSON.stringify(`${characterName} · 上传并添加`)}]`;
    const upload = await $(uploadSelector);
    await upload.waitForEnabled();
    await upload.setValue(fileURLToPath(new URL("../../resources/thelittleprince.png", import.meta.url)));
    let uploadedBinding;
    await browser.waitUntil(async () => {
      uploadedBinding = (await readPage()).reference_bindings.find((binding) => binding.character_name === characterName);
      return uploadedBinding.user_added_assets.length === 1;
    }, { timeout: 30_000, timeoutMsg: "The uploaded user reference was not persisted" });
    assert.deepEqual(uploadedBinding.bound_asset, originalBinding.bound_asset);
    const uploadedAsset = uploadedBinding.user_added_assets[0];
    assert.equal(uploadedAsset.asset_kind, "generated_image");
    assert.deepEqual((await readPage()).reference_discovery, original.reference_discovery);

    await browser.refresh();
    await $(uploadSelector).waitForExist();
    const reloadedBinding = (await readPage()).reference_bindings.find((binding) => binding.character_name === characterName);
    assert.deepEqual(reloadedBinding.user_added_assets, [uploadedAsset]);
    assert.deepEqual(reloadedBinding.bound_asset, originalBinding.bound_asset);
    await browser.setWindowSize(390, 1000);
    const image = await $(`img[alt=${JSON.stringify(`${characterName} 用户添加参考图 1`)}]`);
    await image.scrollIntoView({ block: "center", inline: "nearest" });
    await browser.waitUntil(() => image.execute((node) => node.complete && node.naturalWidth > 0));
    assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
    await browser.saveScreenshot(`${artifacts}/real-comic-reference-upload-390.png`);
    const remove = await $(`button[aria-label=${JSON.stringify(`移除${characterName}第 1 张用户参考图`)}]`);
    await remove.scrollIntoView({ block: "center", inline: "nearest" });
    await remove.click();
    await browser.waitUntil(async () => {
      const binding = (await readPage()).reference_bindings.find((row) => row.character_name === characterName);
      assert.deepEqual(binding.bound_asset, originalBinding.bound_asset);
      return binding.user_added_assets.length === 0;
    }, { timeoutMsg: "Removing a user reference must preserve the system portrait" });
    assert.deepEqual((await readPage()).reference_discovery, original.reference_discovery);
    await browser.refresh();
    await $(uploadSelector).waitForExist();
    assert.equal(await $('button[aria-label^="移除"][aria-label$="张用户参考图"]').isExisting(), false);
  });
});
