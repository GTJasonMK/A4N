import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const byText = (text) => $(`//button[normalize-space()="${text}"]`);
const replaceInput = async (input, value) => {
  await input.click();
  await browser.keys(["Control", "a"]);
  await browser.keys(value);
  assert.equal(await input.getValue(), value);
};

describe("nested editor and input contracts", () => {
  beforeEach(async () => {
    await browser.url("/e2e/ui-presentation/controls.html");
    await byText("编辑场景").waitForDisplayed();
  });

  afterEach(async function () {
    const directory = fileURLToPath(new URL("../../../.dev-runtime/ui-details/", import.meta.url));
    await mkdir(directory, { recursive: true });
    const name = this.currentTest.title.replace(/[^a-z0-9]+/gi, "-").slice(0, 100);
    await browser.saveScreenshot(`${directory}/${process.env.A4N_UI_AUDIT_PHASE ?? "current"}-${name}.png`);
  });

  it("closes only the select popup on Escape inside a dialog", async () => {
    await byText("编辑场景").click();
    const select = await $('[role="dialog"] button[role="combobox"]');
    await select.click();
    await $('[role="listbox"]').waitForDisplayed();
    await browser.keys("Escape");
    await $('[role="listbox"]').waitForExist({ reverse: true });
    assert.equal(await $('[role="dialog"]').isExisting(), true);
    assert.equal(await $('[role="dialog"]').parentElement().getAttribute("data-modal-state"), "entering");
  });

  it("returns to the parent trigger after closing a child dialog", async () => {
    await byText("编辑场景").click();
    await $("#open-character").click();
    await browser.waitUntil(async () => (await $$('[role="dialog"]')).length === 2);
    await browser.keys("Escape");
    await browser.waitUntil(async () => (await $$('[role="dialog"]')).length === 1);
    await browser.waitUntil(async () => await $("#open-character").isFocused());
  });

  it("restores the original trigger after a dialog hands off to its editor", async () => {
    const trigger = await byText("编辑场景");
    await trigger.click();
    await $("#replace-dialog").click();
    await $('//div[@role="dialog" and .//h3[normalize-space()="角色详情"]]').waitForDisplayed();
    await browser.waitUntil(async () => (await $$('[role="dialog"]')).length === 1);
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForExist({ reverse: true });
    assert.equal(await trigger.isFocused(), true, await browser.execute(() => `Focus remained on ${document.activeElement?.outerHTML.slice(0, 300)}`));
  });

  it("restores the trigger of an autofocus dialog in StrictMode", async () => {
    const trigger = await byText("快速命名");
    await trigger.click();
    await $('[role="dialog"]').waitForDisplayed();
    await browser.executeAsync((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    assert.equal(await $('[role="dialog"] input').isFocused(), true);
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForExist({ reverse: true });
    assert.equal(await trigger.isFocused(), true);
  });

  it("preserves autofocus and return focus when both handoff dialogs mount conditionally", async () => {
    const trigger = await byText("切换编辑弹窗");
    await trigger.click();
    await byText("转到编辑器").click();
    const input = await $('input[aria-label="编辑名称"]');
    await input.waitForDisplayed();
    await browser.executeAsync((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    assert.equal(await input.isFocused(), true);
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForExist({ reverse: true });
    assert.equal(await trigger.isFocused(), true);
  });

  it("includes native disclosure controls in the modal Tab cycle", async () => {
    await byText("阅读说明").click();
    const close = await $('[role="dialog"] button[aria-label="关闭"]');
    await browser.waitUntil(async () => await close.isFocused());
    await browser.keys("Tab");
    assert.equal(await $('[role="dialog"] summary').isFocused(), true);
  });

  it("does not focus negative-tabindex tabs when wrapping backwards", async () => {
    await byText("选择章节").click();
    const close = await $('[role="dialog"] button[aria-label="关闭"]');
    await browser.waitUntil(async () => await close.isFocused());
    await browser.keys(["Shift", "Tab"]);
    assert.equal(await $('[role="tab"][aria-selected="true"]').isFocused(), true);
  });

  it("opens the visible select when its label is clicked and exposes errors", async () => {
    await $("#mode-label").click();
    await $('[role="listbox"]').waitForDisplayed();
    assert.equal(await $('button[role="combobox"]').getAttribute("aria-invalid"), "true");
  });

  it("dismisses an open select when it becomes disabled", async () => {
    await $('button[role="combobox"]').click();
    await $('[role="listbox"]').waitForDisplayed();
    await browser.execute(() => [...document.querySelectorAll("button")].find((button) => button.textContent === "切换类型禁用").click());
    await $('[role="listbox"]').waitForExist({ reverse: true });
  });

  it("shows the available choices when opening a populated combobox", async () => {
    await $('input[role="combobox"]').click();
    await $('[role="listbox"]').waitForDisplayed();
    assert.equal((await $$('[role="option"]')).length, 3);
  });

  it("keeps IME confirmation from committing a combobox suggestion", async () => {
    const input = await $('input[role="combobox"]');
    await replaceInput(input, "思");
    await $('[role="listbox"]').waitForDisplayed();
    await input.execute((element) => element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", isComposing: true, bubbles: true, cancelable: true })));
    assert.equal(await input.getValue(), "思");
  });

  it("saves the selected value after leaving, not the prefix before selection", async () => {
    const input = await $('input[role="combobox"]');
    await replaceInput(input, "思");
    await $('[role="option"]').click();
    assert.equal(await $('[data-saved-poses]').getText(), "[]");
    assert.equal(await input.getValue(), "思考");
    await browser.keys("Tab");
    assert.equal(await $('[data-saved-poses]').getText(), '["思考"]');
  });

  it("keeps readonly combobox values unchanged", async () => {
    await $('input[type="checkbox"]').click();
    await $('input[role="combobox"]').click();
    assert.equal(await $('[role="listbox"]').isExisting(), false);
  });

  it("keeps Home and End available for editing text", async () => {
    const input = await $('input[role="combobox"]');
    await input.click();
    await input.execute((element) => element.setSelectionRange(0, 0));
    await browser.keys("End");
    assert.equal(await input.execute((element) => element.selectionStart), 2);
    await browser.keys("Home");
    assert.equal(await input.execute((element) => element.selectionStart), 0);
  });

  it("opens chapter outline actions by keyboard without also opening details", async () => {
    const more = await $('section li button.pyqt-btn-icon');
    await more.execute((element) => element.focus());
    await browser.keys("Enter");
    assert.equal(await $('[data-outline-actions]').getText(), '["actions"]');
  });

  it("keeps quick tools reachable by keyboard while yielding to content", async () => {
    const orb = await $(".a4n-status-bar-orb");
    await browser.waitUntil(async () => (await orb.getAttribute("data-over-control")) === "true");
    await $("#under-orb").execute((element) => element.focus());
    await browser.keys("Tab");
    assert.equal(await orb.isFocused(), true);
    assert.equal(await orb.getCSSProperty("transform").then((property) => property.value), "none");
  });

  it("keeps composer toolbar keyboard actions separate from sending", async () => {
    const choice = await $$('#composer-case .a4n-seg__btn')[1];
    await choice.execute((element) => element.focus());
    await browser.keys("Enter");
    assert.equal(await $('[data-composer-actions]').getText(), "[]");
    assert.equal(await choice.getAttribute("aria-pressed"), "true");
    await $('#composer-case button[aria-haspopup="menu"]').execute((element) => element.focus());
    await browser.keys("Enter");
    await $('[role="menu"]').waitForDisplayed();
    assert.equal(await $('[data-composer-actions]').getText(), "[]");
  });

  it("keeps Enter sending from the message editor and from its send button", async () => {
    await $('#composer-case textarea').click();
    await browser.keys("Enter");
    assert.equal(await $('[data-composer-actions]').getText(), '["send"]');
    await $('#composer-case button[type="submit"]').execute((element) => element.focus());
    await browser.keys("Enter");
    assert.equal(await $('[data-composer-actions]').getText(), '["send","send"]');
  });

  it("allows keyboard cancellation of a running composer task", async () => {
    await $("#running-task").click();
    const cancel = await $('#composer-case form button[title]:not([aria-haspopup])');
    await cancel.execute((element) => element.focus());
    await browser.keys("Enter");
    assert.equal(await $('[data-composer-actions]').getText(), '["cancel"]');
  });
});
