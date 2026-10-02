import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { installUiFixtures, uiFixtures } from "../ui-presentation/fixtures.mjs";
import { visualFixtures } from "../ui-presentation/visual-fixtures.mjs";
import { authoringChapters, authoringWorkspace } from "./fixtures.mjs";

const artifactDir = fileURLToPath(new URL("../../../.dev-runtime/media-authoring/browser/", import.meta.url));
const text = "雨从旧书店的屋檐落下。她推开门。";
const hash = createHash("sha256").update(text).digest("hex");
const projects = visualFixtures.projectRecords.map((project) => ({ ...project, source_snapshot_id: project.root_project_id ? project.id + 10 : null }));
const fixture = {
  ...visualFixtures,
  projectRecords: [...projects, { ...projects.find((project) => project.id === 3), id: 7, title: "第二先导短片" }],
  list_source_snapshot_chapters: authoringChapters(3),
  list_video_adaptation_references: [],
  list_manga_page_prompts: [],
  list_manga_pipeline_statuses: [1, 2].map((chapter_id) => ({ chapter_id, pending: 0, generating: 0, completed: 0, failed: 0, cancelled: 0, pipeline_status: null, stage: null, current: null, total: null, message: null, task_id: null, can_cancel: false, can_resume: false, has_checkpoint: false, checkpoint_valid: false })),
  get_galgame_asset_plan: null,
  preview_media_adaptation_plan_request: { title: "改编请求", workflow: "media_adaptation_plan", steps: [], notes: [], ready_step_count: 0, execution_location: "server" },
  list_chapter_video_prompts: [], list_videos_by_chapter: [], list_video_reference_images: [], list_video_segment_execution_recipes: [],
  get_chapter_video_timeline: { id: 1, revision: 1, items: [] },
  get_chapter_shooting_script: null, get_chapter_shooting_script_unit_progress: null,
  list_scene_profiles: [], list_workflow_runs: [], list_profile_materials: [],
  get_chapter_visual_plan: { confirmed: null, draft: null, scene_choices: [], source: { chapter_number: 1, content: text, content_sha256: hash, title: "旧书店" }, starter_plan: {
    schema_version: "chapter_visual_plan_v3", frame: { width: 1920, height: 1080 },
    points: [{ id: "start", at_ms: 0, label: "开始", image_id: null, description: "", reference_image_ids: [] }, { id: "end", at_ms: 10000, label: "结束", image_id: null, description: "", reference_image_ids: [] }],
    segments: [{ id: "shot-1", title: "镜头 1", start_point_id: "start", end_point_id: "end", prompt: "", scene_binding: null, use_end_point_image: false }],
  } },
  byProject: {
    2: { list_source_snapshot_chapters: authoringChapters(2), get_media_authoring_workspace: authoringWorkspace(2, "manga") },
    5: { list_source_snapshot_chapters: authoringChapters(5), get_media_authoring_workspace: authoringWorkspace(5, "galgame") },
    7: { list_source_snapshot_chapters: authoringChapters(7) },
  },
};

const stage = (label) => $(`//button[@role="tab" and contains(.,"${label}")]`);
const chooseStage = async (label) => {
  const select = await $('[role="combobox"][id$="-select"]');
  if (await select.isDisplayed()) {
    await select.click();
    await $('[role="listbox"]').waitForDisplayed();
    for (const option of await $('[role="listbox"]').$$('[role="option"]')) {
      if ((await option.getText()).includes(label)) {
        await option.click();
        return;
      }
    }
    throw new Error(`Missing stage ${label}`);
  }
  await stage(label).click();
};
const clickText = (text) => $(`//button[normalize-space()="${text}"]`).click();
const intention = () => $('//label[span[normalize-space()="改编目标"]]/textarea');
const diagnostics = () => browser.execute(() => ({ unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors }));

async function checkLayout(name, width) {
  await browser.setWindowSize(width, 1000);
  await browser.waitUntil(async () => await browser.execute(() => document.documentElement.scrollWidth === document.documentElement.clientWidth));
  assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
  await browser.saveScreenshot(`${artifactDir}/${name}-${width}.png`);
}

describe("Derived media authoring stages", () => {
  before(async () => { await mkdir(artifactDir, { recursive: true }); await installUiFixtures(browser, fixture); });
  afterEach(async function () {
    await browser.saveScreenshot(`${artifactDir}/${this.currentTest.title.replace(/[^a-z0-9]/gi, "-")}.png`);
    const state = await diagnostics();
    assert.deepEqual(state.errors, []);
    assert.deepEqual(state.unexpected, []);
  });

  it("keeps a comic draft across cancelled stage and chapter navigation, then saves its version", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/comic-project/2");
    await chooseStage("改编计划");
    await intention().waitForDisplayed();
    assert.equal(await $('[role="tablist"]').$$('[role="tab"]').length, 7);
    assert.match(await $('nav[aria-label="漫画章节"] button[aria-current="true"]').getText(), /旧书店/);
    await intention().setValue("保留我刚补写的犹豫和反应");
    await stage("改编计划").execute((element) => element.focus());
    await browser.keys("ArrowRight");
    await $('[role="dialog"]').waitForDisplayed();
    await clickText("取消");
    assert.equal(await stage("改编计划").getAttribute("aria-selected"), "true");
    assert.equal(await intention().getValue(), "保留我刚补写的犹豫和反应");
    await $('//button[contains(.,"未寄出的信")]').click();
    await $('[role="dialog"]').waitForDisplayed();
    await clickText("取消");
    assert.equal(await intention().getValue(), "保留我刚补写的犹豫和反应");
    await browser.execute(() => {
      const fixture = window.__uiAudit.fixtures;
      const record = fixture.byProject[2].get_media_authoring_workspace.documents[0].record;
      record.document.intention = "保留我刚补写的犹豫和反应";
      record.revision = 2;
      fixture.save_media_authoring_document = structuredClone(record);
      window.__uiAudit.deferred.push("save_media_authoring_document");
    });
    await clickText("保存草稿");
    await browser.waitUntil(async () => (await browser.execute(() => window.__uiAudit.requests)).some((request) => request.command === "save_media_authoring_document"));
    const request = await browser.execute(() => window.__uiAudit.requests.find((request) => request.command === "save_media_authoring_document").body.input);
    assert.equal(request.record_id, 2000);
    assert.equal(request.expected_revision, 1);
    const saveButton = await $('//button[normalize-space()="保存草稿"]');
    const generateButton = await $('//button[normalize-space()="生成计划草稿" or contains(normalize-space(), "重新生成草稿")]');
    await browser.waitUntil(async () => await saveButton.$(".animate-spin").isExisting());
    assert.equal(await generateButton.$(".animate-spin").isExisting(), false);
    await browser.execute(() => {
      const pending = window.__uiAudit.pending.find((item) => item.command === "save_media_authoring_document");
      const workspace = window.__uiAudit.fixtures.byProject[2].get_media_authoring_workspace;
      const saved = { ...workspace.documents[0].record, document: pending.body.input.document, revision: 2 };
      pending.resolve(new Response(JSON.stringify(saved), { headers: { "Content-Type": "application/json" } }));
    });
    await browser.waitUntil(async () => (await $("body").getText()).includes("计划草稿已保存"));
    await checkLayout("comic-plan", 1440);
    await checkLayout("comic-plan", 1024);
    await checkLayout("comic-plan", 390);
    const adopt = await $('//button[normalize-space()="采用此版本"]');
    await adopt.scrollIntoView();
    assert.equal(await adopt.isClickable(), true);
    await browser.saveScreenshot(`${artifactDir}/comic-plan-footer-390.png`);
    await chooseStage("页面提示词");
    assert.equal(await stage("页面提示词").getAttribute("aria-selected"), "true");
  });

  it("shows preview loading on the preview action and opens the returned request", async () => {
    await browser.setWindowSize(390, 1000);
    await browser.url("/comic-project/2");
    const adaptationTab = await stage("改编计划");
    if (await adaptationTab.getAttribute("aria-selected") !== "true") await chooseStage("改编计划");
    await intention().waitForDisplayed();
    await browser.execute(() => { window.__uiAudit.deferred.push("preview_media_adaptation_plan_request"); });
    const previewButton = await $('//button[normalize-space()="查看生成请求"]');
    const generateButton = await $('//button[normalize-space()="生成计划草稿" or contains(normalize-space(), "重新生成草稿")]');
    await previewButton.click();
    await browser.waitUntil(async () => await browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "preview_media_adaptation_plan_request")));
    await browser.waitUntil(async () => await previewButton.$(".animate-spin").isExisting());
    assert.equal(await generateButton.$(".animate-spin").isExisting(), false);
    assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
    await browser.execute(() => {
      const pending = window.__uiAudit.pending.find((item) => item.command === "preview_media_adaptation_plan_request");
      pending.resolve(Response.json(window.__uiAudit.fixtures.preview_media_adaptation_plan_request));
    });
    await $("[role=dialog]").waitForDisplayed();
    await browser.keys("Escape");
    await browser.waitUntil(async () => await browser.execute(() => document.querySelector("[role=dialog]") == null));
  });

  it("clears the previous video's content while the next route project is loading", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/video-project/3");
    await browser.waitUntil(async () => (await $("body").getText()).includes("先导短片"));
    await browser.execute(() => {
      window.__uiAudit.deferred.push("get_project");
      window.history.pushState({}, "", "/video-project/7");
      window.dispatchEvent(new PopStateEvent("popstate", { state: window.history.state }));
    });
    await browser.waitUntil(async () => await browser.execute(() => window.__uiAudit.pending.some((item) => item.command === "get_project" && String(item.body.projectId) === "7")));
    assert.ok(!(await $("body").getText()).includes("先导短片"));
    await browser.execute(() => {
      const project = window.__uiAudit.fixtures.projectRecords.find((item) => item.id === 7);
      window.__uiAudit.deferred = window.__uiAudit.deferred.filter((command) => command !== "get_project");
      for (const pending of window.__uiAudit.pending.filter((item) => item.command === "get_project" && String(item.body.projectId) === "7")) {
        pending.resolve(Response.json(project));
      }
    });
    await browser.waitUntil(async () => (await $("body").getText()).includes("第二先导短片"));
  });

  it("presents seven game stages and keeps image configuration out of asset planning", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/galgame-project/5");
    await chooseStage("改编计划");
    await intention().waitForDisplayed();
    assert.equal(await $('[role="tablist"]').$$('[role="tab"]').length, 7);
    await checkLayout("galgame-plan", 1440);
    await checkLayout("galgame-plan", 1024);
    await checkLayout("galgame-plan", 390);
    await chooseStage("资产规划");
    await $('//h2[normalize-space()="资产规划"]').waitForDisplayed();
    assert.equal(await $('//label[contains(.,"图片尺寸")]').isExisting(), false);
    await chooseStage("检查导出");
    await $('//h2[normalize-space()="检查导出"]').waitForDisplayed();
    assert.equal((await browser.execute(() => window.__uiAudit.commands)).some((command) => command.startsWith("generate_") || command.startsWith("start_")), false);
  });

  it("shows seven video stages with keyboard navigation and an associated panel", async () => {
    await browser.setWindowSize(1440, 1000);
    await browser.url("/video-project/3");
    await $('[role="tablist"]').waitForDisplayed();
    await chooseStage("导演编排");
    await $('.director-preview-surface').waitForDisplayed();
    assert.equal(await $('[role="tablist"]').$$('[role="tab"]').length, 7);
    await checkLayout("video-director", 1440);
    await checkLayout("video-director", 1024);
    await checkLayout("video-director", 390);
    assert.ok(await $('.director-preview-surface').getSize("height") >= 120, "the mobile preview must have visible height");
    await $('[role="combobox"][id$="-select"]').click();
    await $('[role="listbox"]').waitForDisplayed();
    assert.equal((await $('[role="listbox"]').$$('[role="option"]')).length, 7);
    await browser.keys("Escape");
    await browser.setWindowSize(1440, 1000);
    const active = await $('[role="tab"][aria-selected="true"]');
    assert.equal(await $(`#${await active.getAttribute("aria-controls")}`).getAttribute("role"), "tabpanel");
    await active.execute((element) => element.focus());
    await browser.keys("ArrowRight");
    await browser.waitUntil(async () => (await $('[role="tab"][aria-selected="true"]').getText()).includes("剧本"));
    assert.equal(await $('[role="tabpanel"]').getAttribute("aria-labelledby"), await $('[role="tab"][aria-selected="true"]').getAttribute("id"));
  });

  it("shows adopted plans and outdated outputs without hiding saved work", async () => {
    await browser.setWindowSize(1024, 900);
    await browser.url("/comic-project/2");
    await chooseStage("改编计划");
    await intention().waitForDisplayed();
    await browser.execute(() => {
      const workspace = window.__uiAudit.fixtures.byProject[2].get_media_authoring_workspace;
      workspace.documents[0].record.status = "adopted";
    });
    await clickText("刷新");
    await browser.waitUntil(async () => (await stage("改编计划").getText()).includes("已采用"));
    assert.match(await stage("改编计划").getText(), /已采用/);
    await checkLayout("comic-adopted", 1024);
    assert.match(await browser.execute(() => getComputedStyle(document.querySelector("footer")).backgroundColor), /^rgb\(/);
    await checkLayout("comic-adopted", 390);
    await browser.execute(() => {
      const workspace = window.__uiAudit.fixtures.byProject[2].get_media_authoring_workspace;
      workspace.artifacts = [{ kind: "manga_pages", source_chapter_id: 1, count: 5, stale: true }];
    });
    await clickText("刷新");
    await chooseStage("页面提示词");
    await $('//p[contains(.,"这些产物使用了旧计划")]').waitForDisplayed();
    await checkLayout("comic-outdated", 390);
    await checkLayout("comic-outdated", 1024);
    assert.match(await stage("页面提示词").getText(), /需更新/);
  });
});
