import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { login } from "../full-project/user-flow.mjs";

const report = JSON.parse(readFileSync(process.env.A4N_MEDIA_AUTHORING_CLI_REPORT, "utf8"));
const ids = report.ui_project_ids;
const artifacts = process.env.A4N_BROWSER_E2E_ARTIFACT_DIR;
assert.ok(report.success && ids, "Run the CLI acceptance before checking its imported projects");

describe("Media authoring against the real Host", () => {
  before(login);

  for (const [kind, count] of [["comic", 5], ["galgame", 7], ["video", 7]]) {
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
          if (width < 640) {
            const selector = await $('[role="combobox"][id$="-select"]');
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

  it("saves a real comic draft while cancelling stage navigation preserves unsaved text", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url(`/comic-project/${ids.comic}`);
    await $('[role="tablist"]').waitForDisplayed();
    await $('[role="tablist"]').$$('[role="tab"]')[1].click();
    const intention = () => $('//label[span[normalize-space()="改编目标"]]/textarea');
    await intention().waitForDisplayed();
    const edited = `真实界面保存：保留怀表的发现与人物反应 ${Date.now()}`;
    await intention().setValue(edited);
    await $('[role="tab"][aria-selected="true"]').execute((element) => element.focus());
    await browser.keys("ArrowRight");
    await $('[role="dialog"]').waitForDisplayed();
    await browser.waitUntil(async () => await browser.execute(() => document.querySelector('[role="dialog"]')?.contains(document.activeElement)));
    await browser.keys("Tab");
    assert.ok(await browser.execute(() => document.querySelector('[role="dialog"]')?.contains(document.activeElement)));
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForDisplayed({ reverse: true });
    assert.equal(await intention().getValue(), edited);
    await browser.setWindowSize(390, 1000);
    const stageSelector = await $('[role="combobox"][id$="-select"]');
    await stageSelector.execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
    await stageSelector.click();
    await $('[role="listbox"]').waitForDisplayed();
    await $('[role="listbox"]').$$('[role="option"]')[2].click();
    await $('[role="dialog"]').waitForDisplayed();
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForDisplayed({ reverse: true });
    assert.equal(await intention().getValue(), edited);
    const origin = await $('[role="combobox"][aria-label="内容来源"]');
    await origin.execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
    await origin.click();
    await $('[role="listbox"]').waitForDisplayed();
    assert.equal((await $('[role="listbox"]').$$('[role="option"]')).length, 4);
    await browser.saveScreenshot(`${artifacts}/real-comic-origin-menu.png`);
    await browser.keys("Escape");
    await browser.setWindowSize(1440, 1000);
    await $('//button[normalize-space()="保存草稿"]').click();
    await $('[role="tab"][aria-selected="true"]').execute((element) => element.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "instant" }));
    await browser.waitUntil(async () => (await $('[role="tab"][aria-selected="true"]').getText()).includes("草稿"));
    await browser.refresh();
    await intention().waitForDisplayed();
    assert.equal(await intention().getValue(), edited);
    await $('//button[normalize-space()="采用此版本"]').click();
    const next = await $('//button[normalize-space()="下一阶段"]');
    await next.waitForEnabled();
    await browser.saveScreenshot(`${artifacts}/real-comic-adopted.png`);
    await next.click();
    await browser.waitUntil(async () => await $('[role="tablist"]').$$('[role="tab"]')[2].getAttribute("aria-selected") === "true");
  });
});
