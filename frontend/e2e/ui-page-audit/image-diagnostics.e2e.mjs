import assert from "node:assert/strict";
import { mkdir, readdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { installUiFixtures, uiFixtures } from "../ui-presentation/fixtures.mjs";
import seed from "./novel-fixtures.json" with { type: "json" };

const output = fileURLToPath(
  new URL(
    "../../../.dev-runtime/ui-pages/novel/n09-diagnostic-20261011/",
    import.meta.url,
  ),
);
const projectRecords = seed.projectRecords;
const imageCommands = [
  "list_images_by_project",
  "list_images_by_projects",
  "list_images_by_chapter",
];
const timeout = 10_000;
const progress = (message) => console.info(`[n09-diagnostics] ${message}`);

async function clickText(scope, text) {
  const button = await scope.$(
    `.//button[normalize-space(.)=${JSON.stringify(text)}]`,
  );
  await button.waitForDisplayed({ timeout });
  await button.click();
}

describe("N09 raw generation record", function () {
  this.timeout(60_000);

  it("keeps long paths and hashes readable inside a keyboard-scrollable region", async () => {
    await mkdir(output, { recursive: true });
    progress("loading production assets and fixtures");
    const assets = await readdir(
      new URL("../../dist/assets/", import.meta.url),
    );
    const logo = assets.find((name) => /^afn-logo-.*\.svg$/.test(name));
    assert.ok(
      logo,
      "The built A4N logo is available as a same-origin fixture image",
    );
    const filePath = new URL(`/assets/${logo}`, browser.options.baseUrl).href;
    const images = seed.commands.list_images_by_projects.map((image) => ({
      ...image,
      file_path: filePath,
    }));
    const fixtures = {
      ...uiFixtures,
      ...seed,
      ...seed.commands,
      projectRecords,
      list_image_configs: [],
      list_projects: {
        items: projectRecords,
        page: 1,
        page_size: 20,
        total: projectRecords.length,
      },
      ...Object.fromEntries(imageCommands.map((command) => [command, images])),
    };

    await installUiFixtures(browser, fixtures);
    progress("fixtures installed");
    await browser.setWindowSize(1440, 900);
    await browser.url("/novel/1?ui-n09-diagnostics=1");
    progress("project route opened");
    await $(".a4n-page-container").waitForDisplayed({ timeout });
    progress("project mounted");
    await clickText(await $("nav[role=tablist]"), "图像");
    progress("image tab opened");
    const imageButton = await $('button[title="管理图片"]');
    await imageButton.waitForDisplayed({ timeout });
    const galleryState = await browser.execute(() => ({
      imageButtons: document.querySelectorAll('button[title="管理图片"]')
        .length,
      commands: window.__uiAudit?.commands ?? [],
    }));
    assert.ok(galleryState.imageButtons > 0);
    progress(`gallery ready ${JSON.stringify(galleryState)}`);
    await imageButton.click();
    progress("image detail opened");
    const dialog = await $('[role="dialog"]');
    await dialog.waitForDisplayed({ timeout });
    await clickText(dialog, "来源");
    progress("source details opened");
    const identity = await browser.execute((node) =>
      node.querySelector("h3")?.parentElement?.innerText ?? "",
      dialog,
    );
    assert.ok(identity.includes("记录标识: 40001"));
    const metadata = await dialog.$("dl");
    await metadata.waitForDisplayed({ timeout });
    const expected = images.find((image) => image.id === 40001);
    assert.ok((await dialog.getText()).includes("第 1 章"));
    await (await dialog.$("summary")).click();

    const results = [];
    for (const width of [390, 900, 1440]) {
      await browser.setWindowSize(width, 900);
      const region = await dialog.$(
        '[role="region"][aria-label="原始生成记录"]',
      );
      await region.waitForDisplayed({ timeout });
      const rawRecord = JSON.parse(await region.$("pre").getText());
      assert.equal(
        rawRecord.metadata_json.source.content_sha256,
        expected.metadata_json.source.content_sha256,
      );
      const layout = await browser.execute((node) => {
        const pre = node.querySelector("pre");
        const scrollHint = node.closest("details")?.querySelector(
          ".a4n-raw-record-scroll-hint",
        );
        return {
          pageOverflow:
            document.documentElement.scrollWidth -
            document.documentElement.clientWidth,
          dialogOpacity: getComputedStyle(node.closest('[role="dialog"]'))
            .getPropertyValue("--a4n-dialog-opacity")
            .trim(),
          regionTabIndex: node.tabIndex,
          overflowX: getComputedStyle(node).overflowX,
          whiteSpace: getComputedStyle(pre).whiteSpace,
          clientWidth: node.clientWidth,
          scrollWidth: node.scrollWidth,
          horizontalScrollHintVisible: Boolean(
            scrollHint && getComputedStyle(scrollHint).display !== "none",
          ),
          filePath: pre.textContent.includes("file_path"),
          hash: pre.textContent.includes("content_sha256"),
        };
      }, region);
      assert.equal(layout.pageOverflow, 0);
      assert.equal(layout.dialogOpacity, "1");
      assert.equal(layout.regionTabIndex, 0);
      assert.equal(layout.overflowX, "auto");
      assert.equal(layout.whiteSpace, "pre");
      assert.equal(layout.filePath, true);
      assert.equal(layout.hash, true);
      assert.equal(layout.horizontalScrollHintVisible, width < 640);
      if (width === 390) assert.ok(layout.scrollWidth > layout.clientWidth);
      await browser.saveScreenshot(`${output}N09-diagnostics-${width}.png`);
      results.push({ width, ...layout, identity });
      progress(`captured ${width}px`);
    }

    const audit = await browser.execute(() => ({
      errors: window.__uiAudit?.errors ?? [],
      unexpected: window.__uiAudit?.unexpected ?? [],
    }));
    const severeLogs = (await browser.getLogs("browser")).filter(
      (entry) => entry.level === "SEVERE",
    );
    assert.deepEqual(audit.errors, []);
    assert.deepEqual(audit.unexpected, []);
    assert.deepEqual(severeLogs, []);
    await writeFile(
      `${output}report.json`,
      JSON.stringify({ results, audit, severeLogs }, null, 2),
    );
  });
});
