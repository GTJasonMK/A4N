import assert from "node:assert/strict";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";

describe("guest appearance synchronization across login windows", function () {
  this.timeout(30_000);

  let primaryWindow;
  let guestWindow;

  before(async () => {
    primaryWindow = await browser.getWindowHandle();
    await installUiFixtures(browser, visualFixtures);
  });

  afterEach(async () => {
    const handles = await browser.getWindowHandles();
    if (guestWindow && handles.includes(guestWindow)) {
      await browser.switchToWindow(guestWindow);
      await browser.closeWindow();
    }
    guestWindow = undefined;
    if ((await browser.getWindowHandles()).includes(primaryWindow)) {
      await browser.switchToWindow(primaryWindow);
    }
  });

  it("UX-1770 propagates a manual theme choice between two login windows", async () => {
    const url = `/?ui-auth=guest&ui-locale=zh-CN&ui-appearance=light&ui-run=${Date.now()}`;
    await browser.url(url);
    await browser.waitUntil(() => browser.execute(() => document.getElementById("root")?.childElementCount > 0), {
      timeout: 5000,
      timeoutMsg: "Login page React root did not mount",
    });
    const openTools = await $('button[aria-label="展开快捷工具"]');
    await openTools.waitForDisplayed();
    await openTools.click();
    const firstDarkButton = () => $('button[aria-label="切换到暗色主题"]');
    await firstDarkButton().waitForDisplayed();

    const opened = await browser.newWindow(new URL(url, browser.options.baseUrl).href);
    guestWindow = opened.handle;
    const secondDarkButton = await $('button[aria-label="切换到暗色主题"]');
    await secondDarkButton.waitForDisplayed();

    await browser.switchToWindow(primaryWindow);
    await (await firstDarkButton()).click();
    await browser.waitUntil(async () =>
      await $('button[aria-label="切换到亮色主题"]').getAttribute("aria-pressed") === "true",
      { timeout: 5000, timeoutMsg: "The second guest window did not adopt the dark preference" },
    );
    await browser.switchToWindow(primaryWindow);
    await browser.waitUntil(async () => await $('button[aria-label="切换到亮色主题"]').getAttribute("aria-pressed") === "true");

    await browser.switchToWindow(guestWindow);
    await $('button[aria-label="切换到亮色主题"]').click();
    await browser.waitUntil(async () =>
      await $('button[aria-label="切换到暗色主题"]').getAttribute("aria-pressed") === "false",
      { timeout: 5000, timeoutMsg: "The first guest window did not adopt the light preference" },
    );
    await browser.waitUntil(async () => browser.execute(() =>
      localStorage.getItem("a4n:theme-appearance:guest") === "light",
    ));
  });
});
