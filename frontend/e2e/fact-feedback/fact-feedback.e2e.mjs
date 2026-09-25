import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import path from "node:path";

describe("正文事实反馈界面", () => {
  it("显示证据、保存周期并实际执行反馈，窄屏没有横向溢出", async () => {
    await browser.url("/e2e/fact-feedback/");
    await $("h3=正文事实反馈").waitForDisplayed();
    await browser.waitUntil(async () => (await $("body").getText()).includes("待归纳 2/5 章"));
    const input = await $("input[type=number]");
    assert.equal(await input.getAttribute("min"), "1");
    await input.setValue("0");
    assert.equal(await browser.execute(() => document.querySelector("input").checkValidity()), false);
    await input.setValue("3");
    await $("button=保存周期").click();
    await browser.waitUntil(async () => (await $("body").getText()).includes("待归纳 2/3 章"));
    const writes = await browser.execute(() => window.factFeedbackFixture.calls.filter((call) => call.command === "update_project"));
    assert.equal(writes.length, 1);
    assert.equal(writes[0].args.input.fact_feedback_interval_chapters, 3);
    await $("summary=最近 2 项资料变化").click();
    assert.ok((await $("body").getText()).includes("母亲的声音"));
    const directory = process.env.A4N_BROWSER_REPORT_DIR ?? "../data/fact-feedback-browser";
    await mkdir(directory, { recursive: true });
    for (const width of [1440, 1024, 390]) {
      await browser.setWindowSize(width, 1000);
      const size = await browser.execute(() => ({ width: innerWidth, overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth }));
      assert.equal(size.overflow, 0, JSON.stringify(size));
      await browser.saveScreenshot(path.resolve(directory, `facts-${width}.png`));
    }
    await input.click();
    await browser.keys("Tab");
    assert.notEqual(await browser.execute(() => document.activeElement.tagName), "BODY");
    await $("button=立即归纳").click();
    await browser.waitUntil(async () => (await $("body").getText()).includes("待归纳 0/3 章"));
    const runs = await browser.execute(() => window.factFeedbackFixture.calls.filter((call) => call.command === "run_project_fact_feedback"));
    assert.equal(runs.length, 1);
    assert.equal(runs[0].args.input.mode, "consolidate");
  });
});
