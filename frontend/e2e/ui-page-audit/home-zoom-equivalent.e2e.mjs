import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "../ui-presentation/fixtures.mjs";
import { buildPageFixtures } from "./fixtures.mjs";

const runId = new Date().toISOString().replace(/[:.]/g, "-");
const output = fileURLToPath(new URL(
  `../../../.dev-runtime/ui-pages/home-agent/zoom-equivalent-${runId}/`,
  import.meta.url,
));
const report = {
  runId,
  mode: null,
  initialViewport: null,
  nativeZoomWindow: null,
  viewport: { width: 702, height: 393 },
  home: null,
  responsive: [],
  focus: null,
  dialog: null,
};

const runXdotool = (args) => {
  const result = spawnSync("xdotool", args, { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr || "xdotool command failed");
  return result.stdout ?? "";
};

describe("Home page at a 702 x 393 CSS viewport", () => {
  before(async () => {
    await mkdir(output, { recursive: true });
    await installUiFixtures(browser, await buildPageFixtures(browser.options.baseUrl));
  });

  after(async () => {
    if (report.home == null) {
      try {
        report.bootstrap = await browser.execute(() => ({
          url: location.href,
          text: document.body?.innerText,
          hasAuth: localStorage.getItem("a4n:current_access_token") != null,
          fixtureReady: window.__uiAudit != null,
          errors: window.__uiAudit?.errors,
          unexpected: window.__uiAudit?.unexpected,
        }));
        await browser.saveScreenshot(`${output}/bootstrap-failure.png`);
      } catch (error) {
        report.bootstrapError = String(error);
      }
    }
    await writeFile(`${output}/report.json`, JSON.stringify(report, null, 2));
  });

  it("keeps project discovery, create, and keyboard focus usable", async () => {
    const nativeZoomConfigured = process.env.A4N_UI_CONFIGURE_NATIVE_ZOOM === "1";
    const stableDeviceScale = process.env.A4N_UI_CHROME_SCALE_FACTOR === "1";
    await browser.setWindowSize(1440, 1000);
    if (nativeZoomConfigured) {
      await browser.url("chrome://settings/appearance");
      const zoom = await browser.execute(() => {
        const settingsRoot = document.querySelector("settings-ui")?.shadowRoot;
        const main = settingsRoot?.querySelector("settings-main")?.shadowRoot;
        const appearanceIndex = main?.querySelector("settings-appearance-page-index")?.shadowRoot;
        const appearance = appearanceIndex?.querySelector("settings-appearance-page")?.shadowRoot;
        const container = settingsRoot?.querySelector("#container");
        if (container) container.scrollTop = container.scrollHeight;
        const select = appearance?.querySelector("#zoomLevel");
        if (!select) throw new Error("Chrome Settings Page zoom control is unavailable");
        select.value = "2";
        select.dispatchEvent(new Event("change", { bubbles: true, composed: true }));
        return { value: select.value, text: select.options[select.selectedIndex]?.text.trim() };
      });
      assert.deepEqual(zoom, { value: "2", text: "200%" });
    }
    await browser.url("/");
    await $(".a4n-project-row").waitForDisplayed({ timeout: 10_000 });
    const readViewport = () => browser.execute(() => ({
      width: innerWidth,
      height: innerHeight,
      devicePixelRatio,
      visualViewportScale: visualViewport?.scale ?? null,
    }));
    const isNative200 = (viewport) => viewport.width <= 720
      && viewport.visualViewportScale === 1
      && (viewport.devicePixelRatio >= 3.8 || nativeZoomConfigured || stableDeviceScale);
    const initialViewport = await readViewport();
    report.initialViewport = initialViewport;
    const zoomSteps = [];
    if (process.env.A4N_UI_APPLY_NATIVE_ZOOM === "1" && !isNative200(initialViewport)) {
      const windowIds = runXdotool(["search", "--onlyvisible", "--class", "chromium"])
        .trim()
        .split(/\s+/)
        .filter(Boolean);
      const windows = windowIds.map((id) => {
        return {
          id,
          title: runXdotool(["getwindowname", id]).trim(),
        };
      });
      const pageTitle = await browser.getTitle();
      const windowId = windows.findLast((window) => window.title.startsWith(pageTitle))?.id;
      assert.ok(windowId, `The Chromium window for ${JSON.stringify(pageTitle)} must be available for native browser zoom: ${JSON.stringify(windows)}`);
      report.nativeZoomWindow = windows.find((window) => window.id === windowId);
      runXdotool(["windowactivate", "--sync", windowId]);
      assert.equal(runXdotool(["getwindowfocus"]).trim(), windowId, "The browser zoom shortcut must reach the active Chromium window");

      let actualZoom = initialViewport;
      for (const shortcut of ["ctrl+plus", "ctrl+shift+equal"]) {
        for (let step = 0; step < 6; step += 1) {
          runXdotool(["key", "--clearmodifiers", shortcut]);
          await browser.pause(150);
          actualZoom = await readViewport();
          zoomSteps.push({ shortcut, step: step + 1, ...actualZoom });
          if (isNative200(actualZoom)) break;
        }
        if (isNative200(actualZoom)) break;
        runXdotool(["key", "--clearmodifiers", "ctrl+0"]);
        await browser.pause(150);
      }
    }
    const actualViewport = await readViewport();
    const native200 = isNative200(actualViewport);
    report.actualViewport = actualViewport;
    report.zoomSteps = zoomSteps;
    report.native200 = native200;
    report.mode = native200 ? "native-200-percent" : "resized-css-viewport";
    if (process.env.A4N_UI_REQUIRE_NATIVE_ZOOM === "1") {
      assert.equal(native200, true, "The isolated Chrome profile must open at native 200% zoom");
    }
    if (!native200) {
      const browserFrameHeight = await browser.execute(() => outerHeight - innerHeight);
      await browser.setWindowSize(702, 393 + browserFrameHeight);
    }
    await browser.waitUntil(async () => {
      const viewport = await browser.execute(() => ({ width: innerWidth, height: innerHeight }));
      return viewport.width === 702 && viewport.height === 393;
    }, { timeout: 5000, timeoutMsg: "The home layout must be measured at a 702 x 393 CSS viewport" });
    await browser.execute(async () => {
      const { setThemeAppearance } = await import("/src/theme/activeTheme.ts");
      setThemeAppearance("light");
      await document.fonts.ready;
      await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    });

    const home = await browser.execute(() => {
      const rect = (element) => {
        const box = element?.getBoundingClientRect();
        return box ? { left: box.left, top: box.top, right: box.right, bottom: box.bottom, width: box.width, height: box.height } : null;
      };
      const controls = [...document.querySelectorAll("button,input:not([type=hidden]),select,textarea,[role=combobox],summary")]
        .filter((element) => {
          const style = getComputedStyle(element);
          const box = element.getBoundingClientRect();
          return style.display !== "none" && style.visibility !== "hidden" && box.width > 2 && box.height > 2
            && box.bottom > 0 && box.top < innerHeight
            && !element.closest(".a4n-mobile-scroll-rail,.a4n-filter-chips");
        });
      const outside = controls.filter((element) => {
        const box = element.getBoundingClientRect();
        return box.left < -1 || box.right > innerWidth + 1;
      }).map((element) => ({
        label: element.getAttribute("aria-label") || element.textContent?.trim().slice(0, 60),
        box: rect(element),
      }));
      const filterLabels = [...document.querySelectorAll(".a4n-home-filter-bar label")].map((label) => {
        const title = label.querySelector("span");
        const range = document.createRange();
        if (title) range.selectNodeContents(title);
        return {
          text: title?.textContent.trim() ?? "",
          box: rect(label),
          titleRects: [...range.getClientRects()].map((box) => ({ left: box.left, right: box.right, top: box.top, bottom: box.bottom })),
          scrollWidth: label.scrollWidth,
          clientWidth: label.clientWidth,
        };
      });
      return {
        width: innerWidth,
        height: innerHeight,
        devicePixelRatio,
        visualViewportScale: visualViewport?.scale ?? null,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        title: rect(document.querySelector(".a4n-home-heading h1")),
        create: rect(document.querySelector('button[aria-label="创建项目"]')),
        import: rect(document.querySelector('button[aria-label="导入项目"]')),
        accessScope: (() => {
          const trigger = document.querySelector('button[aria-label="访问权限"]');
          const value = trigger?.querySelector(".a4n-themed-select__value");
          return value ? {
            text: value.textContent.trim(),
            clientWidth: value.clientWidth,
            scrollWidth: value.scrollWidth,
          } : null;
        })(),
        filterLabels,
        firstProject: rect(document.querySelector(".a4n-project-row")),
        outside,
        errors: window.__uiAudit.errors,
        unexpected: window.__uiAudit.unexpected,
      };
    });
    report.home = home;
    await browser.saveScreenshot(`${output}/home-702x393.png`);
    if (native200 && process.env.A4N_UI_CAPTURE_NATIVE_WINDOW === "1") {
      const pageTitle = await browser.getTitle();
      const windowIds = runXdotool(["search", "--onlyvisible", "--class", "chromium"])
        .trim()
        .split(/\s+/)
        .filter(Boolean);
      const windowId = windowIds.find((id) => runXdotool(["getwindowname", id]).trim().startsWith(pageTitle));
      assert.ok(windowId, `The Chromium window for ${JSON.stringify(pageTitle)} must be available for a native screenshot`);
      const capture = spawnSync("import", ["-window", windowId, `${output}/home-native-window.png`], { encoding: "utf8" });
      if (capture.error) throw capture.error;
      if (capture.status !== 0) throw new Error(capture.stderr || "Native X11 screenshot capture failed");
    }

    assert.equal(home.width, 702);
    assert.equal(home.height, 393);
    assert.equal(home.overflow, 0, "The home page must not overflow horizontally at this narrow viewport");
    assert.ok(home.title?.width > 0 && home.title.right <= home.width + 1, "The title must stay inside the viewport");
    assert.ok(home.create?.width > 0 && home.create.right <= home.width + 1, "The create action must remain visible");
    assert.ok(home.import?.width > 0 && home.import.right <= home.width + 1, "The import action must remain visible");
    assert.equal(home.accessScope?.text, "全部可访问", "The access filter value must remain fully readable");
    assert.ok(home.accessScope.scrollWidth <= home.accessScope.clientWidth, "The access filter value must not be clipped");
    assert.equal(home.filterLabels.length, 2, "Both project filters must keep visible labels");
    assert.ok(home.filterLabels.every((label) => label.titleRects.length > 0
      && label.titleRects.every((box) => box.left >= -1 && box.right <= home.width + 1)), "Filter labels must remain inside the viewport");
    assert.ok(home.firstProject?.width > 0 && home.firstProject.right <= home.width + 1, "The first project card must stay inside the viewport");
    assert.ok(home.firstProject.top < home.height, "The first project card must remain visible below the filters");
    assert.deepEqual(home.outside, []);
    assert.deepEqual(home.errors, []);
    assert.deepEqual(home.unexpected, []);

    const createButton = await $('button[aria-label="创建项目"]');
    let tabsToCreate = 0;
    while (!await createButton.isFocused() && tabsToCreate < 80) {
      await browser.keys("Tab");
      tabsToCreate += 1;
    }
    assert.equal(await createButton.isFocused(), true, "The create action must remain keyboard reachable");
    const focus = await browser.execute(() => {
      const element = document.activeElement;
      const box = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return {
        label: element.getAttribute("aria-label"),
        box: { left: box.left, top: box.top, right: box.right, bottom: box.bottom },
        outline: style.outlineStyle,
        outlineWidth: style.outlineWidth,
      };
    });
    report.focus = { tabsToCreate, ...focus };
    assert.notEqual(focus.outline, "none");
    assert.ok(parseFloat(focus.outlineWidth) > 0);
    await browser.saveScreenshot(`${output}/home-create-focus-702x393.png`);

    const writesBeforeOpen = await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "create_project").length);
    await createButton.click();
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed({ timeout: 5000 });
    const dialogState = await browser.execute(() => {
      const element = document.querySelector('[role="dialog"]');
      const box = element.getBoundingClientRect();
      return {
        box: { left: box.left, top: box.top, right: box.right, bottom: box.bottom, width: box.width, height: box.height },
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        writes: window.__uiAudit.requests.filter((item) => item.command === "create_project").length,
      };
    });
    report.dialog = dialogState;
    await browser.saveScreenshot(`${output}/create-dialog-702x393.png`);
    assert.ok(dialogState.box.left >= -1 && dialogState.box.right <= 703, "The create dialog must fit the viewport width");
    assert.equal(dialogState.overflow, 0, "Opening create must not introduce horizontal overflow");
    assert.equal(dialogState.writes, writesBeforeOpen, "Opening the dialog must not submit a project");
    const lastOption = await $('//*[@role="dialog"]//button[contains(normalize-space(.),"独立漫画项目")]');
    await lastOption.waitForExist({ timeout: 5000 });
    const dialogBody = await dialog.$('[data-modal-body="1"]');
    await dialogBody.execute((element) => { element.scrollTop = element.scrollHeight; });
    const dialogEnd = await browser.execute(() => {
      const body = document.querySelector('[role="dialog"] [data-modal-body="1"]');
      const option = [...(body?.querySelectorAll("button") ?? [])].find((element) => element.textContent.includes("独立漫画项目"));
      if (!body || !option) throw new Error("The final project type option must remain in the create dialog");
      const bodyBox = body.getBoundingClientRect();
      const optionBox = option.getBoundingClientRect();
      return {
        scrollTop: body.scrollTop,
        scrollHeight: body.scrollHeight,
        clientHeight: body.clientHeight,
        body: { top: bodyBox.top, bottom: bodyBox.bottom },
        option: { top: optionBox.top, bottom: optionBox.bottom },
        bottomGap: bodyBox.bottom - optionBox.bottom,
      };
    });
    report.dialogEnd = dialogEnd;
    await browser.saveScreenshot(`${output}/create-dialog-last-option-702x393.png`);
    assert.ok(dialogEnd.scrollHeight > dialogEnd.clientHeight && dialogEnd.scrollTop > 0, "The create dialog must scroll to later project types");
    assert.ok(dialogEnd.scrollTop >= dialogEnd.scrollHeight - dialogEnd.clientHeight - 1, "The dialog body must reach its true content end");
    assert.ok(dialogEnd.option.top >= dialogEnd.body.top - 1 && dialogEnd.option.bottom <= dialogEnd.body.bottom + 1, "The last project type must remain fully visible inside the dialog");
    assert.ok(dialogEnd.bottomGap >= 12, "The final project card must retain the modal body's 16px bottom padding");
    await browser.keys("Escape");
    await dialog.waitForExist({ reverse: true, timeout: 5000 });
    assert.equal(await createButton.isFocused(), true, "Closing the dialog must return focus to its opener");
    assert.equal(await browser.execute(() => window.__uiAudit.requests.filter((item) => item.command === "create_project").length), writesBeforeOpen);

    const browserFrameHeight = await browser.execute(() => outerHeight - innerHeight);
    const responsiveWidths = report.native200 && !stableDeviceScale ? [390] : [390, 1024];
    for (const width of responsiveWidths) {
      await browser.setWindowSize(report.native200 ? width * 2 + 36 : width, report.native200 ? 1000 : 393 + browserFrameHeight);
      await browser.pause(250);
      const resizedViewport = await browser.execute(() => ({ width: innerWidth, height: innerHeight, outerWidth, outerHeight, devicePixelRatio }));
      console.info(`[home-responsive-viewport] ${JSON.stringify({ requested: width, native200: report.native200, resizedViewport })}`);
      await browser.waitUntil(async () => (await browser.execute(() => innerWidth)) === width, {
        timeout: 5000,
        timeoutMsg: `The home page must be measured at ${width}px; got ${JSON.stringify(resizedViewport)}`,
      });
      const responsive = await browser.execute(() => {
        const bounds = (element) => {
          const box = element?.getBoundingClientRect();
          return box ? { left: box.left, right: box.right, top: box.top, bottom: box.bottom } : null;
        };
        const labels = [...document.querySelectorAll(".a4n-home-filter-bar label")].map((label) => {
          const title = label.querySelector("span");
          const range = document.createRange();
          if (title) range.selectNodeContents(title);
          return {
            text: title?.textContent.trim() ?? "",
            box: bounds(label),
            titleRects: [...range.getClientRects()].map((box) => ({ left: box.left, right: box.right })),
            scrollWidth: label.scrollWidth,
            clientWidth: label.clientWidth,
          };
        });
        const accessValue = document.querySelector('button[aria-label="访问权限"] .a4n-themed-select__value');
        const projectTitle = document.querySelector(".a4n-project-row h3 span");
        const projectTitleStyle = projectTitle ? getComputedStyle(projectTitle) : null;
        const mobileEntryLabel = [...document.querySelectorAll(".a4n-project-row span")]
          .find((element) => element.classList.contains("sm:hidden") && element.textContent.trim());
        const controls = [...document.querySelectorAll("button,input:not([type=hidden]),select,textarea,[role=combobox],summary")]
          .filter((element) => {
            const style = getComputedStyle(element);
            const box = element.getBoundingClientRect();
            return style.display !== "none" && style.visibility !== "hidden" && box.width > 2 && box.height > 2;
          });
        return {
          width: innerWidth,
          height: innerHeight,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          title: bounds(document.querySelector(".a4n-home-heading h1")),
          labels,
          accessValue: accessValue ? {
            text: accessValue.textContent.trim(),
            clientWidth: accessValue.clientWidth,
            scrollWidth: accessValue.scrollWidth,
          } : null,
          mobileEntryLabel: mobileEntryLabel ? {
            text: mobileEntryLabel.textContent.trim(),
            display: getComputedStyle(mobileEntryLabel).display,
          } : null,
          projectTitleClamp: projectTitleStyle ? {
            display: projectTitleStyle.display,
            lines: projectTitleStyle.webkitLineClamp,
          } : null,
          firstProject: bounds(document.querySelector(".a4n-project-row")),
          outside: controls.filter((element) => {
            const box = element.getBoundingClientRect();
            return box.left < -1 || box.right > innerWidth + 1;
          }).map((element) => element.getAttribute("aria-label") || element.textContent?.trim().slice(0, 60)),
        };
      });
      report.responsive.push(responsive);
      await browser.saveScreenshot(`${output}/home-${width}x393.png`);
      assert.equal(responsive.width, width);
      assert.equal(responsive.overflow, 0, `The home page must not overflow at ${width}px`);
      assert.ok(responsive.title?.left >= -1 && responsive.title.right <= width + 1);
      assert.equal(responsive.labels.length, 2);
      assert.ok(responsive.labels.every((label) => label.scrollWidth <= label.clientWidth
        && label.titleRects.length > 0
        && label.titleRects.every((box) => box.left >= -1 && box.right <= width + 1)));
      assert.equal(responsive.accessValue?.text, "全部可访问");
      assert.ok(responsive.accessValue.scrollWidth <= responsive.accessValue.clientWidth);
      if (width === 390) {
        assert.equal(responsive.mobileEntryLabel?.text, "写作", "The primary project entry must have a visible short label on mobile");
        assert.notEqual(responsive.mobileEntryLabel.display, "none");
        assert.deepEqual(responsive.projectTitleClamp, { display: "flow-root", lines: "2" }, "Long mobile project titles must remain capped at two lines");
      }
      assert.ok(responsive.firstProject?.left >= -1 && responsive.firstProject.right <= width + 1);
      assert.deepEqual(responsive.outside, []);
    }
  });
});
