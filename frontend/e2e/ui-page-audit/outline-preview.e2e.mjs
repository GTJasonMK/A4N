import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures, uiFixtures } from "../ui-presentation/fixtures.mjs";
import seed from "./novel-fixtures.json" with { type: "json" };

const output = fileURLToPath(
  new URL("../../../.dev-runtime/ui-pages/novel/", import.meta.url),
);
const nativeProfileDir = process.env.A4N_UI_CHROME_PROFILE_DIR ?? fileURLToPath(
  new URL("../../../.dev-runtime/ui-pages/novel/outline-preview-native-zoom-profile/", import.meta.url),
);
const title = seed.commands.get_project.title;
const skipOutlineReview = process.env.A4N_UI_SKIP_N08 === "1";
const skipImportReview = process.env.A4N_UI_SKIP_N10 === "1";
const widths = process.env.A4N_UI_WIDTHS
  ? process.env.A4N_UI_WIDTHS.split(",").map(Number)
  : [390, 900, 1440];
const results = [];
const timeout = 10_000;
const progress = (message) => console.info(`[outline-preview] ${message}`);
const directRoute = process.env.A4N_UI_DIRECT_ROUTE === "1";
const n11ViewerOnly = process.env.A4N_UI_N11_VIEWER_ONLY === "1";
const skipN11ViewerReview = process.env.A4N_UI_SKIP_N11 === "1";
const skipProfileSummaryReview = process.env.A4N_UI_SKIP_PROFILE_SUMMARY === "1";
const skipObjectReview = process.env.A4N_UI_SKIP_OBJECT_REVIEW === "1";
const nativeZoom = process.env.A4N_UI_NATIVE_ZOOM === "1";
const outlineEvidence =
  process.env.A4N_UI_OUTLINE_EVIDENCE ?? "frontend-design-20261011";
const profileSummaryEvidence =
  process.env.A4N_UI_PROFILE_SUMMARY_EVIDENCE ?? "frontend-design-20261011";
const objectSummaryEvidence = process.env.A4N_UI_OBJECT_SUMMARY_EVIDENCE ?? "frontend-design-20261012";
const objectDetailEvidence = process.env.A4N_UI_OBJECT_DETAIL_EVIDENCE ?? "frontend-design-20261012";

async function clickText(scope, value) {
  const button = await scope.$(
    ".//button[normalize-space(.)=" + JSON.stringify(value) + "]",
  );
  await button.waitForDisplayed({ timeout });
  await button.click();
}

async function buttonByText(scope, value) {
  for (const button of await scope.$$("button")) {
    if ((await button.getText()).trim() === value) return button;
  }
  throw new Error(`Missing visible button: ${value}`);
}

async function setViewport(width, height = 900) {
  if (!nativeZoom) {
    await browser.setWindowSize(width, height);
    return;
  }
  const viewport = await browser.execute(() => ({
    width: innerWidth,
    height: innerHeight,
    devicePixelRatio,
    visualViewportScale: visualViewport?.scale ?? null,
  }));
  assert.deepEqual(viewport, {
    width,
    height: 393,
    devicePixelRatio: 2,
    visualViewportScale: 1,
  });
}

const runXdotool = (args) => {
  const result = spawnSync("xdotool", args, { encoding: "utf8" });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr || "xdotool command failed");
  return result.stdout ?? "";
};

async function captureNativeWindow(name) {
  const title = await browser.getTitle();
  const processes = spawnSync("ps", ["-ww", "-eo", "pid=,args="], {
    encoding: "utf8",
  });
  if (processes.error) throw processes.error;
  if (processes.status !== 0) throw new Error(processes.stderr || "ps failed");
  const chromePids = processes.stdout
    .split(/\r?\n/)
    .flatMap((line) => {
      const match = line.match(/^\s*(\d+)\s+(.+)$/);
      if (!match) return [];
      const [, pid, args] = match;
      return args.includes(`--user-data-dir=${nativeProfileDir}`) && !args.includes("--type=")
        ? [pid]
        : [];
    });
  const windowIds = chromePids.flatMap((pid) =>
    runXdotool(["search", "--onlyvisible", "--pid", pid])
      .trim()
      .split(/\s+/)
      .filter(Boolean),
  );
  const windowId = windowIds.find((id) =>
    runXdotool(["getwindowname", id]).trim().startsWith(title),
  );
  assert.ok(
    windowId,
    `A visible Chromium window for profile ${JSON.stringify(nativeProfileDir)} and title ${JSON.stringify(title)} must exist`,
  );
  runXdotool(["windowactivate", "--sync", windowId]);
  assert.equal(runXdotool(["getwindowfocus"]).trim(), windowId);
  const path = `${output}${outlineEvidence}-N08-native-200-${name}-window.png`;
  const capture = spawnSync("import", ["-window", windowId, path], { encoding: "utf8" });
  if (capture.error) throw capture.error;
  if (capture.status !== 0) throw new Error(capture.stderr || "Native X11 capture failed");
  return path;
}

async function capture(width, stage) {
  const state = await browser.execute(() => {
    const section = document.querySelector("#outline-parts-panel");
    if (!section) throw new Error("The part outline panel is missing");
    const rect = (node) => {
      const bounds = node.getBoundingClientRect();
      return {
        top: bounds.top,
        right: bounds.right,
        bottom: bounds.bottom,
        left: bounds.left,
        width: bounds.width,
        height: bounds.height,
      };
    };
    const header = section.querySelector("header");
    const heading = header?.querySelector("h2");
    const headingBlock = heading?.parentElement;
    const actions = header?.querySelector(":scope > div:last-child");
    return {
      viewportHeight: innerHeight,
      overflow:
        document.documentElement.scrollWidth -
        document.documentElement.clientWidth,
      section: rect(section),
      heading: heading?.textContent?.trim() ?? null,
      headingBlock: headingBlock ? rect(headingBlock) : null,
      actions: actions ? rect(actions) : null,
      headerText: header?.innerText ?? "",
      settingsText: section.querySelector("details")?.innerText ?? "",
      settingsExpanded: section.querySelector("details")?.open ?? null,
      parts: [...section.querySelectorAll("li")].map((item) => ({
        text: item.innerText.trim(),
        rect: rect(item),
      })),
    };
  });
  assert.equal(state.overflow, 0);
  assert.equal(state.heading, "分部大纲");
  assert.ok(state.headerText.includes("高级工具"));
  assert.equal(state.parts.length, 3);
  if (nativeZoom) {
    await captureNativeWindow(`parts-${width}-${stage}`);
  } else {
    await browser.saveScreenshot(
      `${output}${outlineEvidence}-N08-parts-${width}-${stage}.png`,
    );
  }
  results.push({ width, stage, ...state });
  progress(`captured N08 ${width}px ${stage}`);
  return state;
}

async function writeReport() {
  const reportName =
    process.env.A4N_UI_PROFILE_SUMMARY_EVIDENCE
      ? `${profileSummaryEvidence}-profile-summary-${widths.join("-")}-report.json`
      : process.env.A4N_UI_OUTLINE_EVIDENCE
        ? `${outlineEvidence}-outline-import-review-${widths.join("-")}-report.json`
    : process.env.A4N_UI_WIDTHS
      ? `frontend-design-20261011-outline-import-review-${widths.join("-")}-report.json`
      : "frontend-design-20261011-outline-import-review-report.json";
  await writeFile(`${output}${reportName}`, JSON.stringify(results, null, 2));
}

describe("Novel outline and import review visual review", function () {
  this.timeout(120_000);

  afterEach(async () => {
    const hasVisibleDialog = async () => {
      const dialog = await $('[role="dialog"]');
      return (await dialog.isExisting()) && (await dialog.isDisplayed());
    };
    if (!(await hasVisibleDialog())) return;
    await browser.keys("Escape");
    await browser.waitUntil(async () => !(await hasVisibleDialog()), { timeout });
  });

  it("checks outline, import upload, and review acceptance at three widths", async () => {
    await mkdir(output, { recursive: true });
    const projects = seed.projectRecords;
    if (n11ViewerOnly) {
      const project = projects.find((record) => record.id === 1);
      assert.ok(project, "The read-only chapter review needs its project fixture");
      project.my_role = "viewer";
    }
    const character = seed.commands.list_profiles[0];
    delete character.current_state.implicit_attributes.personality;
    const scene = seed.commands.list_scene_profiles[0];
    scene.visual_description = "";
    scene.spatial_layout = scene.base_description;
    scene.atmosphere = scene.base_description;
    scene.current_state_json = { current_location: "北侧入口" };
    const chapterIndex = seed.commands.list_chapters.map((chapter) => {
      const { content, ...metadata } = chapter;
      return { ...metadata, body_character_count: content.length };
    });
    const chapterDetails = Object.fromEntries(seed.commands.list_chapters.map((chapter) => [
      chapter.id,
      { ...seed.byChapter?.[chapter.id], get_chapter: chapter },
    ]));
    const fixtures = {
      ...uiFixtures,
      ...seed,
      ...seed.commands,
      projectRecords: projects,
      list_chapter_index: chapterIndex,
      byChapter: chapterDetails,
      list_projects: {
        items: projects,
        page: 1,
        page_size: 20,
        total: projects.length,
      },
    };
    await installUiFixtures(browser, fixtures);
    progress("fixtures installed");
    await browser.setWindowSize(1440, nativeZoom ? 1000 : 900);
    if (nativeZoom) {
      await browser.url("chrome://settings/appearance");
      await browser.waitUntil(
        async () => {
          try {
            return await browser.execute(() => {
              const settingsRoot = document.querySelector("settings-ui")?.shadowRoot;
              const main = settingsRoot?.querySelector("settings-main")?.shadowRoot;
              const appearanceIndex = main?.querySelector("settings-appearance-page-index")?.shadowRoot;
              const appearance = appearanceIndex?.querySelector("settings-appearance-page")?.shadowRoot;
              return Boolean(appearance?.querySelector("#zoomLevel"));
            });
          } catch {
            return false;
          }
        },
        { timeout, timeoutMsg: "Chrome's native zoom control must load" },
      );
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
    await browser.url(
      directRoute
        ? `/novel/1?ui-outline-review=1${n11ViewerOnly ? "&ui-n11-viewer=1" : ""}`
        : "/?ui-outline-review=1",
    );
    progress("home navigation started");
    try {
      await $(
        directRoute ? ".a4n-page-container" : ".a4n-home-content",
      ).waitForDisplayed({ timeout });
    } catch (error) {
      const pageState = await browser
        .execute(() => ({
          url: location.href,
          readyState: document.readyState,
          title: document.title,
          bodyText: document.body.innerText.slice(0, 1200),
          rootHtml:
            document.querySelector("#root")?.innerHTML.slice(0, 1200) ?? null,
          navigation: (() => {
            const entry = performance.getEntriesByType("navigation")[0];
            return entry
              ? {
                  responseStart: entry.responseStart,
                  domContentLoaded: entry.domContentLoadedEventEnd,
                  load: entry.loadEventEnd,
                }
              : null;
          })(),
          resources: performance
            .getEntriesByType("resource")
            .slice(-20)
            .map((entry) => entry.name),
          errors: window.__uiAudit?.errors ?? [],
          commands: window.__uiAudit?.commands?.slice(-20) ?? [],
          unexpected: window.__uiAudit?.unexpected ?? [],
        }))
        .catch((diagnosticError) => ({
          pageStateError: String(diagnosticError),
        }));
      const browserLogs = await browser
        .getLogs("browser")
        .catch((diagnosticError) => [String(diagnosticError)]);
      console.error(
        "UI mount diagnostics: " + JSON.stringify({ pageState, browserLogs }),
      );
      throw error;
    }
    progress(directRoute ? "target route mounted" : "home mounted");
    if (nativeZoom) {
      const viewport = await browser.execute(() => ({
        width: innerWidth,
        height: innerHeight,
        devicePixelRatio,
        visualViewportScale: visualViewport?.scale ?? null,
      }));
      assert.deepEqual(viewport, {
        width: 702,
        height: 393,
        devicePixelRatio: 2,
        visualViewportScale: 1,
      });
      results.push({ stage: "native-200-viewport", ...viewport });
    }
    if (!directRoute) {
      const row = await $(
        `//div[contains(@class,"a4n-project-row") and .//h3[normalize-space(.)=${JSON.stringify(title)}]]`,
      );
      await row.waitForDisplayed({ timeout });
      progress("target project found");
      await (await row.$('button[aria-label="更多操作"]')).click();
      const menu = await $('[role="menu"]');
      await clickText(menu, "查看档案");
      await browser.waitUntil(
        async () => new URL(await browser.getUrl()).pathname === "/novel/1",
        { timeout },
      );
    }
    progress("project opened");
    const page = await $(".a4n-page-container");
    await page.waitForDisplayed({ timeout });
    progress("project content ready");
    if (!skipOutlineReview) {
      await clickText(await $('nav[role="tablist"]'), "大纲");
      progress("N08 tab selected");
      await clickText(page, "分部大纲");
      progress("parts view selected");
      try {
        await browser.waitUntil(
          async () => (await page.getText()).includes("通道内外的双重档案"),
          { timeout },
        );
      } catch (error) {
        const state = await browser
          .execute(() => ({
            url: location.href,
            selectedTabs: [
              ...document.querySelectorAll('nav[role="tablist"] [role="tab"]'),
            ]
              .filter((tab) => tab.getAttribute("aria-selected") === "true")
              .map((tab) => tab.textContent?.trim()),
            pageText:
              document
                .querySelector(".a4n-page-container")
                ?.innerText.slice(0, 1200) ?? "",
            errors: window.__uiAudit?.errors ?? [],
            commands: window.__uiAudit?.commands ?? [],
            resources: performance
              .getEntriesByType("resource")
              .filter(
                (entry) =>
                  entry.name.includes("/assets/") && entry.name.endsWith(".js"),
              )
              .filter((entry) => entry.responseStatus >= 400)
              .map((entry) => ({
                name: entry.name,
                status: entry.responseStatus,
              })),
          }))
          .catch((diagnosticError) => ({
            diagnosticError: String(diagnosticError),
          }));
        const severeLogs = (await browser.getLogs("browser")).filter(
          (entry) => entry.level === "SEVERE",
        );
        console.error(
          `[outline-preview] N08 mount diagnostics ${JSON.stringify({ state, severeLogs })}`,
        );
        throw error;
      }
      progress("N08 outline opened");

      for (const width of widths) {
        await setViewport(width);
        await browser.execute(() => {
          const page = document.querySelector(".a4n-page-container");
          if (page) page.scrollTop = 0;
        });
        const settings = await $("#outline-parts-panel details");
        if (await settings.execute((element) => element.open)) {
          await (await settings.$("summary")).click();
          await browser.waitUntil(
            () => settings.execute((element) => !element.open),
            { timeout },
          );
        }
        await capture(width, "initial");
        if (!(await settings.execute((element) => element.open))) {
          await (await settings.$("summary")).click();
          await browser.waitUntil(
            () => settings.execute((element) => element.open),
            { timeout },
          );
        }
        const configured = await capture(width, "settings");
        assert.ok(configured.settingsText.includes("查看追加请求"));
        assert.ok(configured.settingsText.includes("追加分部"));
        const lastPart = await (
          await $("#outline-parts-panel")
        ).$("li:last-child");
        await lastPart.scrollIntoView({ block: "center" });
        const last = await capture(width, "last");
        assert.ok(
          last.parts.at(-1).rect.bottom <= last.viewportHeight,
          `The final part must remain reachable at ${width}px`,
        );
      }

      const advancedTrigger = await buttonByText(page, "高级工具");
      await advancedTrigger.click();
      const advancedDialog = await $('[role="dialog"]');
      await advancedDialog.waitForDisplayed({ timeout });
      if (nativeZoom) await browser.pause(250);
      for (const width of widths) {
        await setViewport(width);
        const state = await advancedDialog.execute((node) => {
          const body = node.querySelector('[data-modal-body="1"]');
          const active = document.activeElement;
          const triggers = [
            ...node.querySelectorAll('button[aria-label="查看模型请求"]'),
          ];
          const scrollHint = node.querySelector(
            ".a4n-advanced-tools-scroll-hint",
          );
          const closeButton = node.querySelector('button[title*="Esc"]');
          const closeRect = closeButton?.getBoundingClientRect();
          return {
            overflow:
              document.documentElement.scrollWidth -
              document.documentElement.clientWidth,
            viewport: { width: innerWidth, height: innerHeight },
            bodyScrollTop: body?.scrollTop ?? null,
            bodyScrollHeight: body?.scrollHeight ?? null,
            bodyClientHeight: body?.clientHeight ?? null,
            activeElementText: active?.textContent?.trim().slice(0, 80) ?? null,
            activeElementInsideDialog: Boolean(active && node.contains(active)),
            dialogOpacity: getComputedStyle(node).opacity,
            closeButtonVisible: Boolean(
              closeRect && closeRect.left >= 0 && closeRect.right <= innerWidth
                && closeRect.top >= 0 && closeRect.bottom <= innerHeight,
            ),
            scrollHintVisible: Boolean(
              scrollHint && getComputedStyle(scrollHint).display !== "none",
            ),
            scrollHintText: scrollHint?.textContent?.trim() ?? null,
            visibleTriggers: triggers.filter(
              (trigger) => trigger.getClientRects().length > 0,
            ).length,
            triggerLabels: triggers.map((trigger) => trigger.innerText.trim()),
          };
        });
        assert.equal(state.overflow, 0);
        if (nativeZoom) assert.equal(state.bodyScrollTop, 0);
        assert.equal(state.closeButtonVisible, true);
        assert.equal(state.scrollHintVisible, width < 768);
        if (width < 768) {
          assert.equal(state.scrollHintText, "向下滚动查看更多工具");
        }
        assert.ok(
          state.visibleTriggers > 0,
          `The advanced tools need a visible request label at ${width}px`,
        );
        if (nativeZoom) {
          await captureNativeWindow(`advanced-${width}-initial`);
        } else {
          await browser.saveScreenshot(
            `${output}${outlineEvidence}-N08-advanced-${width}.png`,
          );
        }
        results.push({ width, stage: "advanced-tools", ...state });
        progress(`captured N08 advanced tools ${width}px`);
      }
      for (const width of widths) {
        await setViewport(width);
        const body = await advancedDialog.$('[data-modal-body="1"]');
        await body.execute((node) => { node.scrollTop = 0; });
        const beforeScroll = await body.execute((node) => {
          const bodyRect = node.getBoundingClientRect();
          const isVisible = (text) => {
            const summary = [...node.querySelectorAll("summary")].find(
              (item) => item.textContent?.includes(text),
            );
            if (!summary) throw new Error(`The ${text} section must remain available`);
            const rect = summary.getBoundingClientRect();
            return rect.top >= bodyRect.top && rect.bottom <= bodyRect.bottom;
          };
          return {
            scrollHeight: node.scrollHeight,
            clientHeight: node.clientHeight,
            scrollTop: node.scrollTop,
            overflowY: getComputedStyle(node).overflowY,
            scrollbarWidth: node.offsetWidth - node.clientWidth,
            rewriteVisible: isVisible("覆盖性重写"),
            beatMappingVisible: isVisible("节拍映射"),
          };
        });
        assert.equal(beforeScroll.overflowY, "auto");
        assert.ok(beforeScroll.scrollHeight > beforeScroll.clientHeight);
        for (const [text, key] of [
          ["覆盖性重写", "rewrite"],
          ["节拍映射", "beatMapping"],
        ]) {
          const previousScroll = await body.execute((node) => node.scrollTop);
          await body.execute((node, target) => {
            const summary = [...node.querySelectorAll("summary")].find(
              (item) => item.textContent?.includes(target),
            );
            if (!summary) throw new Error(`The ${target} section must remain available`);
            summary.scrollIntoView({ block: "nearest", inline: "nearest" });
            const bodyRect = node.getBoundingClientRect();
            const summaryRect = summary.getBoundingClientRect();
            const bottomGap = summaryRect.bottom - (bodyRect.bottom - 8);
            if (bottomGap > 0) {
              node.scrollTop = Math.min(
                node.scrollHeight - node.clientHeight,
                node.scrollTop + bottomGap,
              );
            }
          }, text);
          const scrolled = await body.execute((node, target) => {
            const bodyRect = node.getBoundingClientRect();
            const summary = [...node.querySelectorAll("summary")].find(
              (item) => item.textContent?.includes(target),
            );
            if (!summary) throw new Error(`The ${target} section must remain available`);
            const rect = summary.getBoundingClientRect();
            return {
              scrollHeight: node.scrollHeight,
              clientHeight: node.clientHeight,
              scrollTop: node.scrollTop,
              text: summary.textContent?.trim(),
              top: rect.top,
              bottom: rect.bottom,
              visible: rect.top >= bodyRect.top && rect.bottom <= bodyRect.bottom,
            };
          }, text);
          progress(`N08 ${width}px scrolled to ${text}: ${JSON.stringify(scrolled)}`);
          assert.ok(scrolled.scrollTop >= previousScroll);
          assert.equal(scrolled.visible, true, `${text} must be reachable by scrolling`);
          const summary = await body.$(
            `.//summary[contains(normalize-space(.), ${JSON.stringify(text)})]`,
          );
          const disclosure = await summary.execute((node) => ({
            open: node.parentElement?.open ?? null,
            icon: node.querySelector(":scope > svg.lucide-chevron-down"),
          }));
          assert.equal(disclosure.open, false, `${text} starts collapsed`);
          assert.ok(disclosure.icon, `${text} uses the Lucide chevron`);
          const icon = await summary.$("svg.lucide-chevron-down");
          assert.equal(await icon.isDisplayed(), true, `${text} chevron is visible`);
          assert.equal(await icon.getAttribute("aria-hidden"), "true");
          await summary.click();
          assert.equal(
            await summary.execute((node) => node.parentElement?.open ?? null),
            true,
            `${text} opens from its summary`,
          );
          await summary.click();
          assert.equal(
            await summary.execute((node) => node.parentElement?.open ?? null),
            false,
            `${text} closes from its summary`,
          );
          if (nativeZoom) {
            await browser.pause(250);
            await captureNativeWindow(`advanced-${width}-${key}-scrolled`);
          } else {
            await browser.saveScreenshot(
              `${output}${outlineEvidence}-N08-advanced-${width}-${key}-scrolled.png`,
            );
          }
          results.push({ width, stage: `advanced-tools-${key}-scrolled`, beforeScroll, ...scrolled });
        }
      }
      await browser.keys("Escape");
      await browser.waitUntil(
        async () => !(await advancedDialog.isDisplayed()),
        { timeout },
      );
      assert.equal(await advancedTrigger.isFocused(), true);
    }

    if (skipImportReview) {
      const runtimeAudit = await browser.execute(() => ({
        errors: window.__uiAudit?.errors ?? [],
        unexpected: window.__uiAudit?.unexpected ?? [],
      }));
      const severeLogs = (await browser.getLogs("browser")).filter(
        (entry) => entry.level === "SEVERE",
      );
      assert.deepEqual(runtimeAudit.errors, []);
      assert.deepEqual(runtimeAudit.unexpected, []);
      assert.deepEqual(severeLogs, []);
      results.push({ stage: "N08-runtime-audit", runtimeAudit, severeLogs });
      await writeReport();
      return;
    }

    await clickText(await $('nav[role="tablist"]'), "导入/归纳");
    progress("N10 tab opened");
    const readImportState = () =>
      browser.execute(() => ({
        pageText:
          document
            .querySelector(".a4n-page-container")
            ?.innerText.slice(0, 1600) ?? "",
        detailButtons: [
          ...(document.querySelectorAll(".a4n-page-container button") ?? []),
        ]
          .map((button) => button.getAttribute("aria-label"))
          .filter((label) => label?.includes("审核详情")),
        commands: window.__uiAudit?.commands.slice(-20) ?? [],
        errors: window.__uiAudit?.errors ?? [],
        failedAssets: performance
          .getEntriesByType("resource")
          .filter(
            (entry) =>
              entry.name.includes("/assets/") &&
              entry.name.endsWith(".js") &&
              entry.responseStatus >= 400,
          )
          .map((entry) => ({ name: entry.name, status: entry.responseStatus })),
      }));
    try {
      await browser.waitUntil(
        async () =>
          browser.execute(
            () =>
              document
                .querySelector(".a4n-page-container")
                ?.innerText.includes("01-旧书店与车站调查补充材料.md") ?? false,
          ),
        {
          timeout,
          timeoutMsg: "The target review record did not appear in the queue",
        },
      );
    } catch (error) {
      const state = await readImportState();
      const severeLogs = (await browser.getLogs("browser")).filter(
        (entry) => entry.level === "SEVERE",
      );
      console.error(
        `[outline-preview] N10 queue diagnostics ${JSON.stringify({ state, severeLogs })}`,
      );
      throw error;
    }
    progress(`N10 queue state ${JSON.stringify(await readImportState())}`);
    for (const width of widths) {
      await setViewport(width);
      await browser.waitUntil(
        () => browser.execute(() => {
          const rail = document.querySelector('nav[role="tablist"]');
          const active = rail?.querySelector('[role="tab"][aria-selected="true"]');
          const railRect = rail?.getBoundingClientRect();
          const tabRect = active?.getBoundingClientRect();
          return Boolean(railRect && tabRect && tabRect.left >= railRect.left && tabRect.right <= railRect.right);
        }),
        { timeout, timeoutMsg: `The active project tab must remain visible after resizing to ${width}px` },
      );
      const activeTab = await browser.execute(() => {
        const rail = document.querySelector('nav[role="tablist"]');
        const active = rail?.querySelector('[role="tab"][aria-selected="true"]');
        const bounds = active?.getBoundingClientRect();
        const railBounds = rail?.getBoundingClientRect();
        const hasBefore = rail?.parentElement?.getAttribute("data-has-before") === "true";
        return {
          label: active?.textContent?.trim() ?? null,
          visible: Boolean(bounds && railBounds && bounds.left >= railBounds.left && bounds.right <= railBounds.right),
          hasBefore,
          partialAtLeadingCue: Boolean(hasBefore && rail && [...rail.querySelectorAll('[role="tab"]')].some((tab) => {
            if (tab === active) return false;
            const rect = tab.getBoundingClientRect();
            const edge = rail.getBoundingClientRect().left + 40;
            return rect.left < edge - 1 && rect.right > edge + 1;
          })),
        };
      });
      assert.equal(activeTab.label, "导入/归纳");
      assert.equal(activeTab.visible, true);
      if (width < 768 && activeTab.hasBefore) {
        assert.equal(activeTab.partialAtLeadingCue, false, "The scroll cue must not cut a neighboring tab label in half");
      }
      results.push({ width, stage: "active-project-tab-visible", ...activeTab });
      const previewRequestButton = await page.$(
        `.//button[normalize-space(.)=${JSON.stringify("查看分析模型请求")}]`,
      );
      await previewRequestButton.waitForDisplayed({ timeout });
      const state = await previewRequestButton.execute((node) => {
        const bounds = node.getBoundingClientRect();
        return {
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          text: node.textContent?.trim() ?? "",
          button: { left: bounds.left, right: bounds.right, top: bounds.top, bottom: bounds.bottom, height: bounds.height },
        };
      });
      assert.equal(state.overflow, 0);
      assert.equal(state.text, "查看分析模型请求");
      assert.ok(state.button.height >= 32);
      assert.ok(state.button.left >= 0 && state.button.right <= width);
      await browser.saveScreenshot(
        `${output}frontend-design-20261011-N10-import-queue-${width}.png`,
      );
      results.push({ width, stage: "import-queue", ...state });
      const fileInput = await page.$('input[type="file"][accept*=".epub"]');
      const filePicker = await fileInput.execute((node) => {
        const trigger = node.previousElementSibling?.querySelector("button");
        return {
          label: node.getAttribute("aria-label"),
          tabIndex: node.tabIndex,
          triggerTabIndex: trigger?.tabIndex ?? null,
        };
      });
      assert.deepEqual(filePicker, {
        label: "选择导入文件",
        tabIndex: -1,
        triggerTabIndex: 0,
      });
      results.push({ width, stage: "import-file-picker-accessibility", ...filePicker });
      progress(`captured N10 import queue ${width}px`);
    }

    const uploadWidth = 390;
    const uploadFileName = "北线访谈补充-390.md";
    const uploadContent = "Additional station notes for the review workflow.";
    await setViewport(uploadWidth);
    const uploadedReview = await browser.execute((fileName, id) => {
      const fixtures = window.__uiAudit.fixtures;
      const source = fixtures.list_import_queue[0];
      const review = {
        ...structuredClone(source),
        id,
        filename: fileName,
        review_status: "pending",
        analysis_result: null,
        updated_at: "2026-10-14T01:00:00Z",
      };
      fixtures.import_project_text_upload = [review];
      fixtures.list_import_queue = [...fixtures.list_import_queue, review];
      window.__uiAudit.deferred.push("import_project_text_upload");
      return review;
    }, uploadFileName, 590);
    await browser.execute((fileName, content) => {
      const input = document.querySelector('input[type="file"][accept*=".epub"]');
      if (!input) throw new Error("The project import file input is missing");
      const transfer = new DataTransfer();
      transfer.items.add(new File([content], fileName, { type: "text/markdown" }));
      input.files = transfer.files;
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }, uploadFileName, uploadContent);
    const importAction = await buttonByText(page, "开始导入");
    await importAction.waitForEnabled({ timeout });
    await importAction.click();
    await browser.waitUntil(
      () => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "import_project_text_upload")),
      { timeout, timeoutMsg: "The selected text file must reach the upload request" },
    );
    const uploadRequest = await browser.execute(() => {
      const request = window.__uiAudit.requests.findLast((item) => item.command === "import_project_text_upload");
      return {
        input: request?.body.input ?? null,
        decoded: request ? atob(request.body.input.data_base64) : null,
      };
    });
    assert.equal(uploadRequest.input.project_id, 1);
    assert.equal(uploadRequest.input.file_name, uploadFileName);
    assert.equal(uploadRequest.decoded, uploadContent);
    const uploadBusy = await importAction.execute((button) => ({
      disabled: button.disabled,
      label: button.textContent?.trim() ?? "",
      spinnerVisible: Boolean(button.querySelector("svg.animate-spin")),
      overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    }));
    assert.equal(uploadBusy.disabled, true);
    assert.ok(uploadBusy.label.includes("导入中"));
    assert.equal(uploadBusy.spinnerVisible, true);
    assert.equal(uploadBusy.overflow, 0);
    await browser.saveScreenshot(`${output}frontend-design-20261014-N10-import-390-uploading.png`);
    results.push({ width: uploadWidth, stage: "import-uploading", ...uploadBusy, fileName: uploadFileName });
    await browser.execute((review) => {
      const pending = window.__uiAudit.pending.findLast((item) => item.command === "import_project_text_upload");
      window.__uiAudit.deferred = window.__uiAudit.deferred.filter((command) => command !== "import_project_text_upload");
      pending.resolve(Response.json([review]));
    }, uploadedReview);
    const uploadedRow = await page.$(`button[aria-label*=${JSON.stringify(uploadFileName)}]`);
    await uploadedRow.waitForDisplayed({ timeout });
    await browser.waitUntil(
      () => browser.execute(() => document.body.innerText.includes("已创建 1 个审核项")),
      { timeout, timeoutMsg: "The upload result must announce how many review items were created" },
    );
    const uploadComplete = await browser.execute(() => {
      const input = document.querySelector('input[type="file"][accept*=".epub"]');
      const trigger = input?.previousElementSibling?.querySelector("button");
      return {
        inputValue: input?.value ?? null,
        triggerTitle: trigger?.getAttribute("title") ?? null,
        rowVisible: [...document.querySelectorAll(".a4n-page-container button")]
          .some((button) => button.getAttribute("aria-label")?.includes("北线访谈补充-390.md")),
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      };
    });
    assert.equal(uploadComplete.inputValue, "");
    assert.equal(uploadComplete.triggerTitle, "选择导入文件");
    assert.equal(uploadComplete.rowVisible, true);
    assert.equal(uploadComplete.overflow, 0);
    await browser.saveScreenshot(`${output}frontend-design-20261014-N10-import-390-uploaded.png`);
    results.push({ width: uploadWidth, stage: "import-uploaded", ...uploadComplete, fileName: uploadFileName });

    const reviewButton = await $(
      'button[aria-label="查看 01-旧书店与车站调查补充材料.md 的审核详情"]',
    );
    await reviewButton.waitForDisplayed({ timeout });
    await reviewButton.click();
    const review = await $("aside");
    await review.waitForDisplayed({ timeout });
    progress("N10 review detail opened");
    const reviewSettings = await review.$("details");
    await (await reviewSettings.$("summary")).click();
    await browser.waitUntil(
      () => reviewSettings.execute((element) => element.open),
      { timeout },
    );

    for (const width of widths) {
      await setViewport(width);
      await review.scrollIntoView({ block: "start" });
      const state = await browser.execute(() => {
        const panel = document.querySelector("aside");
        const field = panel?.querySelector("textarea");
        const action = [...(panel?.querySelectorAll("button") ?? [])].find(
          (button) => button.textContent?.trim() === "保存并接受",
        );
        const label = field?.id
          ? panel?.querySelector(`label[for="${field.id}"]`)
          : null;
        return {
          overflow:
            document.documentElement.scrollWidth -
            document.documentElement.clientWidth,
          fieldId: field?.id ?? null,
          labelText: label?.textContent?.trim() ?? null,
          disabled: action?.disabled ?? null,
          reason: [...(panel?.querySelectorAll('[role="status"]') ?? [])]
            .map((status) => status.textContent?.trim() ?? "")
            .find((text) => text.includes("修改分析结构")) ?? null,
        };
      });
      assert.equal(state.overflow, 0);
      assert.ok(state.fieldId);
      assert.equal(state.labelText, "分析结构");
      assert.equal(state.disabled, true);
      assert.equal(state.reason, "修改分析结构后即可保存并接受。");
      await browser.saveScreenshot(
        `${output}frontend-design-20261011-N10-review-detail-${width}.png`,
      );
      results.push({ width, stage: "review-detail", ...state });
      progress(`captured N10 ${width}px`);
    }

    const reviewJson = await review.$("textarea");
    const editedAnalysis = JSON.parse(await reviewJson.getValue());
    editedAnalysis.review_notes = "已核对来源与正文后接受。";
    await reviewJson.setValue(JSON.stringify(editedAnalysis, null, 2));
    const acceptAction = await buttonByText(review, "保存并接受");
    await acceptAction.waitForEnabled({ timeout });
    for (const width of widths) {
      await setViewport(width);
      await acceptAction.scrollIntoView({ block: "center" });
      const ready = await acceptAction.execute((button) => ({
        disabled: button.disabled,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        bounds: (() => {
          const rect = button.getBoundingClientRect();
          return { left: rect.left, right: rect.right, top: rect.top, bottom: rect.bottom };
        })(),
      }));
      assert.equal(ready.disabled, false);
      assert.equal(ready.overflow, 0);
      assert.ok(ready.bounds.left >= 0 && ready.bounds.right <= width);
      await browser.saveScreenshot(`${output}frontend-design-20261014-N10-review-${width}-ready.png`);
      results.push({ width, stage: "review-ready-to-accept", ...ready });
    }
    await setViewport(390);
    await acceptAction.scrollIntoView({ block: "center" });
    const acceptedReview = await browser.execute((analysisResult) => {
      const fixtures = window.__uiAudit.fixtures;
      const current = fixtures.list_import_queue.find((item) => item.id === 501);
      const updated = {
        ...current,
        analysis_result: analysisResult,
        review_status: "accepted",
        updated_at: "2026-10-14T01:05:00Z",
      };
      fixtures.accept_import_review = updated;
      fixtures.list_import_queue = fixtures.list_import_queue.map((item) => item.id === updated.id ? updated : item);
      window.__uiAudit.deferred.push("accept_import_review");
      return updated;
    }, editedAnalysis);
    await acceptAction.click();
    await browser.waitUntil(
      () => browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "accept_import_review")),
      { timeout, timeoutMsg: "The edited candidate must reach the accept request" },
    );
    const acceptanceRequest = await browser.execute(() =>
      window.__uiAudit.requests.findLast((item) => item.command === "accept_import_review")?.body.input ?? null,
    );
    assert.deepEqual(acceptanceRequest, { review_id: 501, analysis_result: editedAnalysis });
    const acceptBusy = await acceptAction.execute((button) => ({
      disabled: button.disabled,
      label: button.textContent?.trim() ?? "",
      spinnerVisible: Boolean(button.querySelector("svg.animate-spin")),
      focusInsideReview: document.querySelector("aside")?.contains(document.activeElement) ?? false,
    }));
    assert.equal(acceptBusy.disabled, true);
    assert.ok(acceptBusy.label.includes("保存中"));
    assert.equal(acceptBusy.spinnerVisible, true);
    await browser.saveScreenshot(`${output}frontend-design-20261014-N10-review-390-saving.png`);
    await browser.execute((updated) => {
      const pending = window.__uiAudit.pending.findLast((item) => item.command === "accept_import_review");
      window.__uiAudit.deferred = window.__uiAudit.deferred.filter((command) => command !== "accept_import_review");
      pending.resolve(Response.json(updated));
    }, acceptedReview);
    await browser.waitUntil(
      () => review.execute((panel) => panel.querySelector('[role="status"]')?.textContent?.trim() === "已接受"),
      { timeout, timeoutMsg: "The accepted candidate status must be announced in its review context" },
    );
    const acceptanceComplete = await browser.execute(() => {
      const panel = document.querySelector("aside");
      const active = document.activeElement;
      return {
        accepted: panel?.querySelector('[role="status"]')?.textContent?.trim() === "已接受",
        focusInsideReview: panel?.contains(active) ?? false,
        activeTag: active?.tagName ?? null,
        activeText: active?.textContent?.trim() ?? "",
        duplicateSuccessToast: document.body.innerText.includes("修改已保存并接受"),
        outline: active ? {
          style: getComputedStyle(active).outlineStyle,
          width: getComputedStyle(active).outlineWidth,
        } : null,
        overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
      };
    });
    assert.equal(acceptanceComplete.accepted, true);
    assert.equal(acceptanceComplete.duplicateSuccessToast, false);
    assert.equal(acceptanceComplete.overflow, 0);
    assert.equal(acceptanceComplete.focusInsideReview, true, "Accepting must keep keyboard focus in the review context");
    assert.deepEqual(acceptanceComplete.outline, { style: "solid", width: "2px" }, "The focused review title must visibly confirm keyboard context");
    await browser.saveScreenshot(`${output}frontend-design-20261014-N10-review-390-accepted.png`);
    results.push({ width: 390, stage: "review-accepted", ...acceptanceComplete });

    const runtimeAudit = await browser.execute(() => ({
      errors: window.__uiAudit?.errors ?? [],
      unexpected: window.__uiAudit?.unexpected ?? [],
      commands: window.__uiAudit?.commands ?? [],
    }));
    const browserLogs = await browser.getLogs("browser");
    const severeBrowserLogs = browserLogs.filter(
      (entry) => entry.level === "SEVERE",
    );
    assert.deepEqual(
      runtimeAudit.errors,
      [],
      "The reviewed workflow must not raise browser JavaScript errors",
    );
    assert.deepEqual(
      runtimeAudit.unexpected,
      [],
      "The reviewed workflow must use declared UI fixtures only",
    );
    assert.deepEqual(
      severeBrowserLogs,
      [],
      "The reviewed workflow must not log browser resource failures",
    );
    results.push({
      stage: "runtime-audit",
      ...runtimeAudit,
      severeBrowserLogs,
    });
    await writeReport();
  });

  (skipN11ViewerReview ? it.skip : it)("checks read-only chapter navigation and long body reading at three widths", async () => {
    if (!n11ViewerOnly) {
      const project = seed.projectRecords.find((record) => record.id === 1);
      assert.ok(project, "The read-only chapter review needs its project fixture");
      project.my_role = "viewer";
      await browser.execute(() => {
        const record = window.__uiAudit.fixtures.projectRecords.find((item) => item.id === 1);
        if (!record) throw new Error("The project fixture is missing");
        record.my_role = "viewer";
      });
    }

    for (const width of widths) {
      await setViewport(width);
      if (!n11ViewerOnly) await browser.url(`/novel/1?ui-n11-viewer=${width}`);
      const page = await $(".a4n-page-container");
      try {
        await page.waitForDisplayed({ timeout });
      } catch (error) {
        const state = await browser.execute(() => ({
          url: location.href,
          title: document.title,
          bodyText: document.body.innerText.slice(0, 1600),
          rootHtml: document.querySelector("#root")?.innerHTML.slice(0, 1600) ?? null,
          errors: window.__uiAudit?.errors ?? [],
          unexpected: window.__uiAudit?.unexpected ?? [],
          commands: window.__uiAudit?.commands ?? [],
          failedAssets: performance.getEntriesByType("resource")
            .filter((entry) => entry.name.includes("/assets/") && entry.name.endsWith(".js") && entry.responseStatus >= 400)
            .map((entry) => ({ name: entry.name, status: entry.responseStatus })),
        })).catch((diagnosticError) => ({ diagnosticError: String(diagnosticError) }));
        const severeBrowserLogs = (await browser.getLogs("browser"))
          .filter((entry) => entry.level === "SEVERE");
        console.error(`[outline-preview] N11 viewer mount diagnostics ${JSON.stringify({ state, severeBrowserLogs })}`);
        throw error;
      }
      await clickText(await $('nav[role="tablist"]'), "章节");
      await browser.waitUntil(
        () => page.execute((node) => node.innerText.includes("雨夜来信")),
        { timeout },
      );

      const directory = await page.execute((node) => {
        const rows = [...node.querySelectorAll("li button")];
        const rect = (element) => {
          const bounds = element.getBoundingClientRect();
          return {
            left: bounds.left,
            right: bounds.right,
            top: bounds.top,
            bottom: bounds.bottom,
            width: bounds.width,
            height: bounds.height,
          };
        };
        return {
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          rows: rows.map((row) => ({ text: row.innerText.trim(), rect: rect(row) })),
          importActions: [...node.querySelectorAll("button")]
            .filter((button) => button.textContent?.trim() === "导入章节").length,
        };
      });
      assert.equal(directory.overflow, 0);
      assert.equal(directory.rows.length, 3);
      assert.equal(directory.importActions, 0);
      assert.ok(directory.rows.every(({ rect }) => rect.height >= 44 && rect.left >= 0 && rect.right <= width));
      await browser.saveScreenshot(
        `${output}frontend-design-20261011-N11-viewer-${width}-chapters.png`,
      );
      results.push({ width, stage: "N11-viewer-chapters", ...directory });

      const firstRow = await page.$("li button");
      await firstRow.click();
      const preview = await $('[role="dialog"]');
      await preview.waitForDisplayed({ timeout });
      await browser.waitUntil(
        () => preview.execute((node) => node.querySelector("article")?.textContent === window.__uiAudit.fixtures.byChapter[1].get_chapter.content),
        { timeout },
      );
      const title = await preview.$("h3").getText();
      assert.ok(title.includes("第 1 章") && title.includes("雨夜来信"));
      const firstChapter = seed.commands.list_chapters[0];
      assert.equal(
        await preview.$("article").getText(),
        firstChapter.content,
      );
      const modalState = await preview.execute((node) => {
        const body = node.querySelector('[data-modal-body="1"]');
        return {
          opacity: getComputedStyle(node).getPropertyValue("--a4n-dialog-opacity").trim(),
          bodyClientHeight: body?.clientHeight ?? 0,
          bodyScrollHeight: body?.scrollHeight ?? 0,
        };
      });
      assert.equal(modalState.opacity, "1");
      await browser.saveScreenshot(
        `${output}frontend-design-20261011-N11-viewer-${width}-chapter-1.png`,
      );
      const readingEnd = await preview.execute((node) => {
        const body = node.querySelector('[data-modal-body="1"]');
        const article = node.querySelector("article");
        if (!body || !article) throw new Error("The read-only chapter body is missing");
        body.scrollTop = body.scrollHeight;
        const viewport = body.getBoundingClientRect();
        const content = article.getBoundingClientRect();
        return {
          scrollRange: body.scrollHeight - body.clientHeight,
          articleBottom: content.bottom,
          viewportBottom: viewport.bottom,
          tailVisible: content.bottom <= viewport.bottom + 1,
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        };
      });
      assert.ok(readingEnd.scrollRange > 0, "The long chapter must scroll inside the dialog");
      assert.equal(readingEnd.tailVisible, true);
      assert.equal(readingEnd.overflow, 0);
      await browser.saveScreenshot(
        `${output}frontend-design-20261011-N11-viewer-${width}-chapter-1-last.png`,
      );
      results.push({ width, stage: "N11-viewer-chapter-1", title, modalState, readingEnd });

      await browser.keys("Escape");
      await browser.waitUntil(async () => !(await preview.isDisplayed()), { timeout });
      assert.equal(await firstRow.isFocused(), true);
      const secondRow = await page.$("li:nth-child(2) button");
      await secondRow.click();
      const secondPreview = await $('[role="dialog"]');
      await secondPreview.waitForDisplayed({ timeout });
      await browser.waitUntil(
        () => secondPreview.execute((node) => node.querySelector("article")?.textContent === window.__uiAudit.fixtures.byChapter[2].get_chapter.content),
        { timeout },
      );
      const secondTitle = await secondPreview.$("h3").getText();
      assert.ok(secondTitle.includes("第 2 章") && secondTitle.includes("纸上的地图"));
      assert.equal(
        await secondPreview.$("article").getText(),
        seed.commands.list_chapters[1].content,
      );
      await browser.saveScreenshot(
        `${output}frontend-design-20261011-N11-viewer-${width}-chapter-2.png`,
      );
      await browser.keys("Escape");
      await browser.waitUntil(async () => !(await secondPreview.isDisplayed()), { timeout });
    }

    const runtimeAudit = await browser.execute(() => ({
      errors: window.__uiAudit.errors,
      unexpected: window.__uiAudit.unexpected,
      importRequests: window.__uiAudit.requests.filter((request) => request.command === "import_chapter"),
    }));
    assert.deepEqual(runtimeAudit.errors, []);
    assert.deepEqual(runtimeAudit.unexpected, []);
    assert.deepEqual(runtimeAudit.importRequests, []);
    results.push({ stage: "N11-viewer-runtime-audit", ...runtimeAudit });
    await writeReport();
  });

  (skipObjectReview ? it.skip : it)("keeps object card summary and use text distinct when visual details are absent", async () => {
    const profile = seed.commands.list_object_profiles[0];
    await browser.url("/novel/1?ui-object-summary-review=1");
    await browser.refresh();
    const page = await $(".a4n-page-container");
    await page.waitForDisplayed({ timeout });
    await browser.execute((profileId) => {
      const record = window.__uiAudit.fixtures.list_object_profiles.find((item) => item.id === profileId);
      if (!record) throw new Error("The object profile fixture is missing");
      record.visual_description = "";
    }, profile.id);

    for (const width of widths) {
      await setViewport(width);
      await clickText(await $('nav[role="tablist"]'), "物体");
      await browser.waitUntil(
        () => page.execute((node) => node.innerText.includes("旧怀表")),
        { timeout },
      );
      const card = await page.$("li");
      const text = await card.getText();
      assert.equal(text.split(profile.story_usage).length - 1, 1);
      assert.ok(text.includes(profile.location_context));
      const overflow = await browser.execute(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      assert.equal(overflow, 0);
      const headerAction = await page.execute((node) => {
        const header = [...node.querySelectorAll("section > header")]
          .find((element) => element.querySelector("h2")?.textContent?.trim() === "物体档案");
        const button = header?.querySelector('button[aria-haspopup="menu"]');
        if (!header || !button) throw new Error("The object archive action must remain visible");
        const headerRect = header.getBoundingClientRect();
        const buttonRect = button.getBoundingClientRect();
        return {
          headerCenter: headerRect.left + headerRect.width / 2,
          buttonCenter: buttonRect.left + buttonRect.width / 2,
        };
      });
      assert.ok(headerAction.buttonCenter > headerAction.headerCenter);
      await browser.saveScreenshot(`${output}${objectSummaryEvidence}-N04-object-summary-${width}.png`);
      results.push({
        width,
        stage: "N04-object-summary-fallback",
        storyUsageOccurrences: text.split(profile.story_usage).length - 1,
        locationContextVisible: text.includes(profile.location_context),
        headerAction,
        overflow,
      });
    }
    await writeReport();
    await writeFile(
      `${output}${objectSummaryEvidence}-N04-object-summary-report.json`,
      JSON.stringify(results.filter((item) => item.stage === "N04-object-summary-fallback"), null, 2),
    );
  });

  (skipObjectReview ? it.skip : it)("keeps object details readable and the final image action reachable", async () => {
    const profile = seed.commands.list_object_profiles[0];
    await browser.url("/novel/1?ui-object-detail-review=1");
    await browser.refresh();
    const page = await $(".a4n-page-container");
    await page.waitForDisplayed({ timeout });

    const detailResults = [];
    for (const width of widths) {
      await setViewport(width);
      await clickText(await $('nav[role="tablist"]'), "物体");
      await browser.waitUntil(
        () => page.execute((node, name) => node.innerText.includes(name), profile.name),
        { timeout },
      );
      const card = await page.$('.//li[.//button[starts-with(@aria-label, "旧怀表 ·")]]');
      const trigger = await card.$('button[aria-label^="旧怀表 ·"]');
      await trigger.click();
      const detail = await $('[role="dialog"]');
      await detail.waitForDisplayed({ timeout });
      await browser.pause(220);
      const detailText = await detail.getText();
      assert.ok(detailText.includes(profile.story_usage));
      assert.ok(detailText.includes(profile.location_context));
      const surface = await detail.execute((node) => {
        const style = getComputedStyle(node);
        const channels = style.backgroundColor.match(/[\d.]+/g) ?? [];
        return {
          token: style.getPropertyValue("--a4n-dialog-opacity").trim(),
          alpha: Number(channels[3] ?? 1),
          animationOpacity: Number(style.opacity),
        };
      });
      assert.equal(surface.token, "1", "Object details must override the theme opacity locally");
      assert.equal(surface.alpha, 1, "Object details must have a fully opaque background");
      assert.equal(surface.animationOpacity, 1);
      await browser.saveScreenshot(`${output}${objectDetailEvidence}-object-detail-${width}-initial.png`);

      const actionState = await detail.execute((node) => {
        const body = node.querySelector('[data-modal-body="1"]');
        const action = [...node.querySelectorAll("button")]
          .find((button) => button.textContent?.trim().includes("生成素材图"));
        const sizeError = node.querySelector('[role="alert"]');
        if (!body || !action) throw new Error("The object image action must remain available in the detail dialog");
        body.scrollTop = body.scrollHeight;
        const bodyRect = body.getBoundingClientRect();
        const actionRect = action.getBoundingClientRect();
        return {
          visibleAtScrollEnd: actionRect.top >= bodyRect.top - 1 && actionRect.bottom <= bodyRect.bottom + 1,
          disabledWithUnavailableSize: action.disabled,
          sizeUnavailableReason: sizeError?.textContent?.trim() ?? null,
          pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        };
      });
      assert.equal(actionState.visibleAtScrollEnd, true);
      assert.equal(actionState.disabledWithUnavailableSize, true);
      assert.ok(actionState.sizeUnavailableReason?.includes("配置"));
      assert.equal(actionState.pageOverflow, 0);
      await browser.saveScreenshot(`${output}${objectDetailEvidence}-object-detail-${width}-last.png`);
      await browser.keys("Escape");
      await browser.waitUntil(async () => !(await detail.isDisplayed()), { timeout });
      assert.equal(await trigger.isFocused(), true, "Escape must return focus to the opening object card");
      detailResults.push({ width, surface, actionState, focusReturnedAfterEscape: await trigger.isFocused() });
    }
    await writeFile(
      `${output}${objectDetailEvidence}-object-detail-${widths.join("-")}-report.json`,
      JSON.stringify(detailResults, null, 2),
    );
  });

  (skipProfileSummaryReview ? it.skip : it)("keeps character and scene summaries distinct and labels structured fields", async () => {
    const character = seed.commands.list_profiles[0];
    const scene = seed.commands.list_scene_profiles[0];
    const characterSummary = character.current_state.explicit_attributes.background;
    const sceneSummary = scene.base_description;
    await browser.url("/novel/1?ui-profile-summary-review=1");
    await browser.refresh();
    const page = await $(".a4n-page-container");
    await page.waitForDisplayed({ timeout });

    for (const width of widths) {
      await setViewport(width);
      await clickText(await $('nav[role="tablist"]'), "角色");
      await browser.waitUntil(
        () => page.execute((node, name) => node.innerText.includes(name), character.name),
        { timeout },
      );
      const characterCard = await page.$('.//li[.//button[starts-with(@aria-label, "林岚 ·")]]');
      const characterText = await characterCard.getText();
      assert.equal(characterText.split(characterSummary).length - 1, 1);
      assert.ok(characterText.includes("第三站台"));
      assert.equal(characterText.includes("current_location"), false);

      const characterOverflow = await browser.execute(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      assert.equal(characterOverflow, 0);
      await browser.saveScreenshot(`${output}${profileSummaryEvidence}-profile-summary-character-${width}.png`);

      await clickText(await $('nav[role="tablist"]'), "场景");
      await browser.waitUntil(
        () => page.execute((node, name) => node.innerText.includes(name), scene.name),
        { timeout },
      );
      const sceneCard = await page.$('.//li[.//button[starts-with(@aria-label, "旧书店 ·")]]');
      const sceneText = await sceneCard.getText();
      assert.equal(sceneText.split(sceneSummary).length - 1, 1);
      assert.ok(sceneText.includes("未记录空间布局"));
      const sceneOverflow = await browser.execute(
        () => document.documentElement.scrollWidth - document.documentElement.clientWidth,
      );
      assert.equal(sceneOverflow, 0);
      await browser.saveScreenshot(`${output}${profileSummaryEvidence}-profile-summary-scene-${width}.png`);

      const sceneTrigger = await sceneCard.$('button[aria-label^="旧书店 ·"]');
      await sceneTrigger.click();
      const detail = await $('[role="dialog"]');
      await detail.waitForDisplayed({ timeout });
      await browser.pause(220);
      const detailText = await detail.getText();
      const surface = await detail.execute((node) => {
        const style = getComputedStyle(node);
        const channels = style.backgroundColor.match(/[\d.]+/g) ?? [];
        return {
          token: style.getPropertyValue("--a4n-dialog-opacity").trim(),
          alpha: Number(channels[3] ?? 1),
          animationOpacity: Number(style.opacity),
        };
      });
      assert.equal(surface.token, "1", "Scene details must override the theme opacity locally");
      assert.equal(surface.alpha, 1, "Scene details must have a fully opaque background");
      assert.equal(surface.animationOpacity, 1, "The modal enter animation must finish before visual review");
      assert.ok(detailText.includes("当前位置"));
      assert.ok(detailText.includes("北侧入口"));
      assert.equal(detailText.includes("current_location"), false);
      await browser.saveScreenshot(`${output}${profileSummaryEvidence}-profile-structured-label-${width}.png`);
      const actionState = await detail.execute((node) => {
        const body = node.querySelector('[data-modal-body="1"]');
        const action = [...node.querySelectorAll("button")]
          .find((button) => button.textContent?.trim().includes("生成素材图"));
        const sizeError = node.querySelector('[role="alert"]');
        if (!body || !action) throw new Error("The scene image action must remain available in the detail dialog");
        body.scrollTop = body.scrollHeight;
        const bodyRect = body.getBoundingClientRect();
        const actionRect = action.getBoundingClientRect();
        return {
          visibleAtScrollEnd: actionRect.top >= bodyRect.top - 1 && actionRect.bottom <= bodyRect.bottom + 1,
          disabledWithUnavailableSize: action.disabled,
          sizeUnavailableReason: sizeError?.textContent?.trim() ?? null,
          pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        };
      });
      assert.equal(actionState.visibleAtScrollEnd, true, "The scene image action must be reachable at the end of the modal content");
      assert.equal(actionState.disabledWithUnavailableSize, true, "Image generation remains disabled without a valid image size");
      assert.ok(actionState.sizeUnavailableReason?.includes("配置"), "The disabled image action must have a visible reason");
      assert.equal(actionState.pageOverflow, 0);
      await browser.saveScreenshot(`${output}${profileSummaryEvidence}-profile-actions-${width}.png`);
      await browser.keys("Escape");
      await browser.waitUntil(async () => !(await detail.isDisplayed()), { timeout });
      assert.equal(await sceneTrigger.isFocused(), true, "Escape must return focus to the scene card");

      results.push({
        width,
        stage: "profile-summary-and-labels",
        characterSummaryOccurrences: characterText.split(characterSummary).length - 1,
        characterOverflow,
        sceneSummaryOccurrences: sceneText.split(sceneSummary).length - 1,
        sceneOverflow,
        structuredLabelVisible: detailText.includes("当前位置"),
        dialogSurface: surface,
        actionState,
        focusReturnedAfterEscape: await sceneTrigger.isFocused(),
      });
    }
    await writeReport();
  });
});
