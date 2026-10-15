import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "./fixtures.mjs";
import { buildPageFixtures } from "../ui-page-audit/fixtures.mjs";

const directory = fileURLToPath(new URL("../../../.dev-runtime/ui-design-20261010/visual/", import.meta.url));
const cases = [
  ["home", "/", ".a4n-project-row"],
  ["notifications", "/notifications?ui-creator=1", "h1"],
  ["writing", "/write/1?chapter=2", 'textarea[aria-label="章节正文"]'],
  ["settings", "/settings#llm", ".a4n-section-heading"],
  ["theme", "/settings#theme", '[aria-label="外观偏好"]'],
  ["prompt", "/settings#prompt", ".a4n-settings-layout"],
  ["agents", "/agents", 'textarea[aria-label="Agent 消息"]'],
  ["inspiration", "/inspiration/1", 'textarea[aria-label="输入灵感对话内容"]'],
  ["project", "/novel/1", ".a4n-page-header h1"],
  ["comic", "/comic-project/2", ".a4n-page-header h1"],
  ["video", "/video-project/3", ".a4n-page-header h1"],
  ["galgame", "/galgame-project/5", ".a4n-page-header h1"],
  ["canvas", "/canvas-project/6", ".react-flow__node"],
  ["auth", "/?ui-auth=off", ".a4n-auth-card input"],
  ["home-empty", "/?ui-state=empty", ".a4n-empty-state"],
  ["home-error", "/?ui-state=error", ".a4n-home-content [role=alert]"],
  ["home-loading", "/?ui-state=loading", ".a4n-project-skeleton-row"],
  ["home-menu", "/", ".a4n-project-row"],
  ["project-dialog", "/", ".a4n-project-row"],
  ["video-source-dialog", "/", ".a4n-project-row"],
  ["video-title-dialog", "/", ".a4n-project-row"],
  ["settings-editor", "/settings#llm", ".a4n-section-heading"],
  ["home-en", "/?ui-locale=en-US", ".a4n-project-row"],
  ["settings-en", "/settings?ui-locale=en-US#llm", ".a4n-section-heading"],
  ["writing-en", "/write/1?chapter=2&ui-locale=en-US", ".a4n-writing-editor"],
];
const report = [];
let fixtureData;

const inlineImageFixture = (id) => {
  const palettes = [
    ["#20383b", "#314b49", "#f3d894"],
    ["#35404d", "#56626a", "#dca974"],
    ["#33433f", "#53624e", "#e1bd83"],
  ];
  const [wall, glass, light] = palettes[id % palettes.length];
  const lampX = 170 + (id % 5) * 74;
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360"><rect width="640" height="360" fill="${wall}"/><path d="M0 250h640v110H0z" fill="#172a2d"/><rect x="76" y="42" width="488" height="242" fill="#8c4b2b"/><rect x="96" y="62" width="448" height="202" fill="${glass}"/><path d="M112 220h416v24H112z" fill="#855233"/><circle cx="${lampX}" cy="104" r="34" fill="${light}"/><path d="M${lampX - 21} 135h42l14 84h-70z" fill="#c17a3d"/></svg>`;
  return `data:image/svg+xml,${encodeURIComponent(svg)}`;
};

const inlineImageFixtures = (value) => {
  if (typeof value === "string") {
    const match = value.match(/\/image-(\d+)\.svg(?:\?.*)?$/);
    return match ? inlineImageFixture(Number(match[1])) : value;
  }
  if (Array.isArray(value)) return value.map(inlineImageFixtures);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, inlineImageFixtures(item)]));
  }
  return value;
};

const measure = () => {
  const visible = (element) => {
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    return style.display !== "none" && style.visibility !== "hidden" && box.width > 2 && box.height > 2 && box.bottom > 0 && box.top < innerHeight;
  };
  const controls = [...document.querySelectorAll("button,input:not([type=hidden]),textarea,[role=combobox],summary")].filter(visible);
  const images = [...document.images].filter(visible);
  const outside = controls.filter((element) => {
    // These surfaces deliberately scroll/pan inside their own bounded viewport.
    if (element.closest(".react-flow__viewport,.a4n-mobile-scroll-rail,.a4n-filter-chips")) return false;
    const box = element.getBoundingClientRect();
    return box.left < -1 || box.right > innerWidth + 1;
  }).map((element) => ({ label: element.getAttribute("aria-label") || element.textContent?.slice(0, 60), width: element.getBoundingClientRect().width }));
  const heading = document.querySelector(".a4n-section-heading");
  const toolbarTitleOverlaps = [];
  if (heading) {
    const text = document.createRange();
    text.selectNodeContents(heading);
    for (const button of heading.closest("header").querySelectorAll("button")) {
      const box = button.getBoundingClientRect();
      if ([...text.getClientRects()].some((ink) => ink.left < box.right && ink.right > box.left && ink.top < box.bottom && ink.bottom > box.top)) {
        toolbarTitleOverlaps.push(button.textContent);
      }
    }
  }
  const colorChannels = (value) => value.match(/[\d.]+/g).map(Number);
  const luminance = (channels) => channels.reduce((sum, channel, index) => {
    const value = channel / 255;
    const linear = value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
    return sum + [0.2126, 0.7152, 0.0722][index] * linear;
  }, 0);
  const contrastFor = (className) => {
    const sample = document.createElement("span");
    sample.className = className;
    sample.style.cssText = "position:fixed;visibility:hidden;pointer-events:none";
    document.body.append(sample);
    const style = getComputedStyle(sample);
    const foreground = colorChannels(style.color);
    const surface = colorChannels(style.backgroundColor);
    const backdrop = colorChannels(getComputedStyle(document.body).backgroundColor);
    sample.remove();
    const background = surface.slice(0, 3).map((channel, index) =>
      channel * (surface[3] ?? 1) + backdrop[index] * (1 - (surface[3] ?? 1)),
    );
    const foregroundLuminance = luminance(foreground);
    const backgroundLuminance = luminance(background);
    return Number((
      (Math.max(foregroundLuminance, backgroundLuminance) + 0.05) /
      (Math.min(foregroundLuminance, backgroundLuminance) + 0.05)
    ).toFixed(2));
  };
  const tertiaryContrast = contrastFor("bg-book-bg-tertiary text-book-text-muted");
  const statusContrast = Object.fromEntries([
    ["success", "text-success bg-success-bg"],
    ["error", "text-error bg-error-bg"],
    ["warning", "text-warning bg-warning-bg"],
    ["info", "text-info bg-info-bg"],
  ].map(([status, className]) => [status, contrastFor(className)]));
  return {
    viewport: { width: innerWidth, height: innerHeight },
    theme: document.documentElement.dataset.themeMode,
    tertiaryContrast,
    statusContrast,
    composer: document.querySelector(".agent-monitor-composer")?.getBoundingClientRect().toJSON() ?? null,
    overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    outside,
    toolbarTitleOverlaps,
    controls: controls.length,
    visibleImages: images.length,
    brokenImages: images
      .filter((image) => !image.complete || image.naturalWidth === 0)
      .map((image) => image.currentSrc || image.src),
    unexpected: window.__uiAudit.unexpected,
    errors: window.__uiAudit.errors,
    text: document.body.innerText,
  };
};

describe("A4N visual coverage across themes and viewports", () => {
  before(async () => {
    await mkdir(directory, { recursive: true });
    fixtureData = inlineImageFixtures(await buildPageFixtures(browser.options.baseUrl));
    await installUiFixtures(browser, fixtureData);
  });

  after(async () => {
    await writeFile(`${directory}/report.json`, JSON.stringify(report, null, 2));
  });

  for (const [name, route, ready] of cases) {
    it(`renders ${name} in light/dark at desktop, tablet, compact and phone widths`, async () => {
      await browser.setWindowSize(1440, 1000);
      await browser.url(route);
      await $(ready).waitForDisplayed({ timeout: 10_000 });
      if (name === "home-menu") {
        await $('button[aria-label="更多操作"]').click();
        await $('[role="menu"]').waitForDisplayed();
      } else if (name === "project-dialog") {
        await $('//button[normalize-space()="创建项目"]').click();
        await $('[role="dialog"]').waitForDisplayed();
      } else if (name === "video-source-dialog" || name === "video-title-dialog") {
        await $('//button[normalize-space()="创建项目"]').click();
        const creationDialog = await $('[role="dialog"]');
        await creationDialog.waitForDisplayed();
        await creationDialog.$('.//button[.//span[normalize-space(.)="视频项目"]]').click();
        const sourceDialog = await $('//*[@role="dialog" and .//*[normalize-space(.)="选择视频来源项目"]]');
        await sourceDialog.waitForDisplayed({ timeout: 10_000 });
        await sourceDialog.$('button[role="combobox"][aria-label="文本项目来源"]').waitForDisplayed({ timeout: 10_000 });
        const selectedSource = await sourceDialog.$('button[role="combobox"][aria-label="文本项目来源"]').getText();
        const textProject = fixtureData.projectRecords.find((project) => project.project_kind === "text");
        assert.ok(textProject);
        assert.equal(selectedSource.trim(), textProject.title);
        if (name === "video-title-dialog") {
          await sourceDialog.$('.//button[normalize-space(.)="下一步"]').click();
          const titleDialog = await $('//*[@role="dialog" and .//*[normalize-space(.)="创建视频项目"]]');
          await titleDialog.waitForDisplayed({ timeout: 10_000 });
        }
      } else if (name === "settings-editor") {
        await $('//button[contains(.,"新增 LLM 配置")]').click();
        await $('input[role="combobox"]').waitForDisplayed();
      }
      for (const width of [1440, 1024, 702, 390]) {
        await browser.setWindowSize(width, 1000);
        for (const mode of ["light", "dark"]) {
          await browser.execute(async (appearance) => {
            const { setThemeAppearance } = await import("/src/theme/activeTheme.ts");
            setThemeAppearance(appearance);
            await document.fonts.ready;
            await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
          }, mode);
          // Covers the 360ms transition/stagger window without waiting for media timelines.
          await browser.pause(450);
          const file = `a4n-20261010-${name}-${width}-${mode}`;
          await browser.saveScreenshot(`${directory}/${file}.png`);
          const result = await browser.execute(measure);
          report.push({ name, route, mode, file: `${file}.png`, ...result });
          assert.equal(result.viewport.width, width, `${file}: actual viewport width`);
          assert.equal(result.overflow, 0, `${file}: document overflow`);
          assert.equal(result.theme, mode, `${file}: theme must finish applying`);
          assert.ok(result.tertiaryContrast >= 4.5, `${file}: muted text on tertiary surface must meet WCAG AA`);
          for (const [status, contrast] of Object.entries(result.statusContrast)) {
            assert.ok(contrast >= 4.5, `${file}: ${status} text on its status surface must meet WCAG AA`);
          }
          assert.deepEqual(result.outside, [], `${file}: clipped controls`);
          assert.deepEqual(result.toolbarTitleOverlaps, [], `${file}: title overlaps actions`);
          assert.deepEqual(result.unexpected, [], `${file}: missing fixture`);
          assert.deepEqual(result.errors, [], `${file}: browser error`);
          if (["comic", "video", "galgame"].includes(name)) {
            assert.deepEqual(result.brokenImages, [], `${file}: visible media image failed to decode`);
          }
          assert.doesNotMatch(result.text, /\p{Extended_Pictographic}/u, `${file}: UI emoji`);
          if (name === "agents") {
            assert.ok(result.composer && result.composer.bottom <= result.viewport.height + 1, `${file}: composer must remain in the viewport`);
          }
          if (name === "inspiration") {
            const latestOption = await browser.execute(() => {
              const input = document.querySelector('textarea[aria-label="输入灵感对话内容"]');
              const viewport = input?.closest("footer")?.parentElement?.querySelector(".overflow-y-auto.custom-scrollbar");
              const last = viewport?.querySelector("button.inspiration-option-card:last-of-type");
              if (!viewport || !last) throw new Error("The latest inspiration option is missing");
              const viewportRect = viewport.getBoundingClientRect();
              const optionRect = last.getBoundingClientRect();
              return {
                atLatest: viewport.scrollTop >= viewport.scrollHeight - viewport.clientHeight - 1,
                fullyVisible: optionRect.top >= viewportRect.top && optionRect.bottom <= viewportRect.bottom,
                bottomGap: Number((viewportRect.bottom - optionRect.bottom).toFixed(2)),
              };
            });
            assert.equal(latestOption.atLatest, true, `${file}: following the latest message should survive viewport reflow`);
            assert.equal(latestOption.fullyVisible, true, `${file}: the latest suggestion must not sit behind the composer`);
            assert.ok(latestOption.bottomGap >= 12, `${file}: the latest suggestion needs breathing room before the composer`);
            report.push({ name, route, mode, stage: "latest-option-visible", ...latestOption, width });
          }
          if (process.env.A4N_LAYOUT_AUDIT_SCRIPT && width === 390 && ["home", "notifications", "settings", "agents", "auth", "home-menu", "project-dialog", "video-source-dialog", "video-title-dialog", "settings-editor"].includes(name)) {
            const audit = await readFile(process.env.A4N_LAYOUT_AUDIT_SCRIPT, "utf8");
            const findings = await browser.execute(new Function(audit + "\nreturn __audit({});"));
            await browser.saveScreenshot(`${directory}/${file}-annotated.png`);
            report[report.length - 1].audit = findings;
            await browser.execute(() => document.getElementById("__auditOverlay")?.remove());
          }
        }
        if (name === "inspiration" && [702, 390].includes(width)) {
          const bottomState = await browser.execute(() => {
            const composer = document.querySelector('textarea[aria-label="输入灵感对话内容"]');
            const viewport = composer?.closest("footer")?.parentElement?.querySelector(".overflow-y-auto.custom-scrollbar");
            if (!viewport) throw new Error("The inspiration message viewport is missing");
            const options = [...viewport.querySelectorAll("button.inspiration-option-card")];
            if (options.length === 0) throw new Error("The inspiration response has no suggestion cards");
            const restoreScrollTop = viewport.scrollTop;
            viewport.scrollTop = viewport.scrollHeight;
            const viewportRect = viewport.getBoundingClientRect();
            const lastOptionRect = options.at(-1).getBoundingClientRect();
            return {
              width: innerWidth,
              optionCount: options.length,
              reachedScrollEnd: viewport.scrollTop >= viewport.scrollHeight - viewport.clientHeight - 1,
              lastOptionFullyVisible: lastOptionRect.top >= viewportRect.top && lastOptionRect.bottom <= viewportRect.bottom,
              lastOptionBottomGap: Number((viewportRect.bottom - lastOptionRect.bottom).toFixed(2)),
              overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
              restoreScrollTop,
            };
          });
          assert.equal(bottomState.optionCount, 3);
          assert.equal(bottomState.reachedScrollEnd, true);
          assert.equal(bottomState.lastOptionFullyVisible, true);
          assert.ok(bottomState.lastOptionBottomGap >= 12, "The final suggestion must clear the scroll viewport edge");
          assert.equal(bottomState.overflow, 0);
          const file = `a4n-20261010-inspiration-${width}-options-bottom-dark`;
          await browser.saveScreenshot(`${directory}/${file}.png`);
          report.push({ name, route, mode: "dark", stage: "last-option-scroll", file: `${file}.png`, ...bottomState });
          await browser.execute((scrollTop) => {
            const composer = document.querySelector('textarea[aria-label="输入灵感对话内容"]');
            const viewport = composer?.closest("footer")?.parentElement?.querySelector(".overflow-y-auto.custom-scrollbar");
            if (viewport) viewport.scrollTop = scrollTop;
          }, bottomState.restoreScrollTop);
        }
      }
    });
  }

  it("honors reduced motion in CSS and the interactive logo", async () => {
    await browser.sendCommandAndGetResult("Emulation.setEmulatedMedia", {
      features: [{ name: "prefers-reduced-motion", value: "reduce" }],
    });
    try {
      await browser.setWindowSize(1440, 1000);
      await browser.url("/");
      const logo = await $("canvas[role=img]");
      await logo.waitForDisplayed();
      await browser.waitUntil(async () => (await browser.execute(() => window.__uiAudit.draws.attached)) > 0);
      const before = await browser.execute(() => window.__uiAudit.draws.attached);
      await logo.moveTo();
      await browser.pause(200);
      const result = await browser.execute(() => ({
        prefersReducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
        draws: window.__uiAudit.draws,
        transition: getComputedStyle(document.querySelector(".pyqt-btn-primary")).transitionDuration,
      }));
      assert.equal(result.prefersReducedMotion, true);
      assert.equal(result.draws.attached, before);
      assert.equal(result.draws.detached, 0);
      assert.ok(result.transition.split(",").every((duration) => parseFloat(duration) === 0));
      report.push({ name: "reduced-motion", ...result });
      await browser.saveScreenshot(`${directory}/a4n-20261010-reduced-motion.png`);
    } finally {
      await browser.sendCommandAndGetResult("Emulation.setEmulatedMedia", { features: [] });
    }
  });
});
