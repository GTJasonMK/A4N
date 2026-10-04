import assert from "node:assert/strict";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures } from "./fixtures.mjs";

const directory = fileURLToPath(new URL("../../../.dev-runtime/ui-design-20260925/visual/", import.meta.url));
import { visualFixtures } from "./visual-fixtures.mjs";

const cases = [
  ["home", "/", ".a4n-project-row"],
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
  ["canvas", "/video-project/6/studio", ".react-flow__node"],
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

const measure = () => {
  const visible = (element) => {
    const style = getComputedStyle(element);
    const box = element.getBoundingClientRect();
    return style.display !== "none" && style.visibility !== "hidden" && box.width > 2 && box.height > 2 && box.bottom > 0 && box.top < innerHeight;
  };
  const controls = [...document.querySelectorAll("button,input:not([type=hidden]),textarea,[role=combobox],summary")].filter(visible);
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
  return {
    viewport: { width: innerWidth, height: innerHeight },
    theme: document.documentElement.dataset.themeMode,
    composer: document.querySelector(".agent-monitor-composer")?.getBoundingClientRect().toJSON() ?? null,
    overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
    outside,
    toolbarTitleOverlaps,
    controls: controls.length,
    unexpected: window.__uiAudit.unexpected,
    errors: window.__uiAudit.errors,
    text: document.body.innerText,
  };
};

describe("iOS visual coverage with the original palette", () => {
  before(async () => {
    await mkdir(directory, { recursive: true });
    await installUiFixtures(browser, visualFixtures);
  });

  after(async () => {
    await writeFile(`${directory}/report.json`, JSON.stringify(report, null, 2));
  });

  for (const [name, route, ready] of cases) {
    it(`renders ${name} in light/dark at desktop, tablet and phone widths`, async () => {
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
        const textProject = visualFixtures.projectRecords.find((project) => project.project_kind === "text");
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
      for (const width of [1440, 1024, 390]) {
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
          const file = `ios-20260925-${name}-${width}-${mode}`;
          await browser.saveScreenshot(`${directory}/${file}.png`);
          const result = await browser.execute(measure);
          report.push({ name, route, mode, file: `${file}.png`, ...result });
          assert.equal(result.viewport.width, width, `${file}: actual viewport width`);
          assert.equal(result.overflow, 0, `${file}: document overflow`);
          assert.equal(result.theme, mode, `${file}: theme must finish applying`);
          assert.deepEqual(result.outside, [], `${file}: clipped controls`);
          assert.deepEqual(result.toolbarTitleOverlaps, [], `${file}: title overlaps actions`);
          assert.deepEqual(result.unexpected, [], `${file}: missing fixture`);
          assert.deepEqual(result.errors, [], `${file}: browser error`);
          assert.doesNotMatch(result.text, /\p{Extended_Pictographic}/u, `${file}: UI emoji`);
          if (name === "agents") {
            assert.ok(result.composer && result.composer.bottom <= result.viewport.height + 1, `${file}: composer must remain in the viewport`);
          }
          if (process.env.A4N_LAYOUT_AUDIT_SCRIPT && width === 390 && ["home", "settings", "agents", "auth", "home-menu", "project-dialog", "video-source-dialog", "video-title-dialog", "settings-editor"].includes(name)) {
            const audit = await readFile(process.env.A4N_LAYOUT_AUDIT_SCRIPT, "utf8");
            const findings = await browser.execute(new Function(audit + "\nreturn __audit({});"));
            await browser.saveScreenshot(`${directory}/${file}-annotated.png`);
            report[report.length - 1].audit = findings;
            await browser.execute(() => document.getElementById("__auditOverlay")?.remove());
          }
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
      await browser.saveScreenshot(`${directory}/ios-20260925-reduced-motion.png`);
    } finally {
      await browser.sendCommandAndGetResult("Emulation.setEmulatedMedia", { features: [] });
    }
  });
});
