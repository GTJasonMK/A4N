import assert from "node:assert/strict";

const project = {
  id: 1, title: "山海之间", description: "关于记忆与归途的故事", project_kind: "text",
  project_type: "novel_long", status: "writing", lifecycle_gate: "active", my_role: "owner",
  root_project_id: null, root_project_title: null, collaboration_root_project_id: 1,
  source_mode: "self", source_sync_status: "current", derived_chapter_scope: null,
  owner_username: "写作者", member_count: 1, pending_invite_count: 0, current_chapter: 2,
  target_chapters: 12, total_chapters: 12, chapters_per_part: null,
  updated_at: "2026-09-11T12:00:00Z", progress: { label: "写作中", detail: null },
};
const projects = [project, {
  ...project, id: 2, title: "山海之间 · 漫画", project_kind: "comic",
  root_project_id: 1, root_project_title: project.title, source_mode: "derived",
}, {
  ...project, id: 3, title: "山海之间 · 先导短片", project_kind: "video",
  root_project_id: 1, root_project_title: project.title, source_mode: "derived",
  lifecycle_gate: "resetting",
}];
const chapters = ["旧书店", "未寄出的信"].map((title, index) => ({
  id: index + 1, project_id: 1, chapter_number: index + 1, title,
  content: `第 ${index + 1} 章 ${title}\n\n雨从旧书店的屋檐落下。`, word_count: 28,
  status: "draft", outline_id: null, analysis_data: null, style_tags: null, summary: null,
  created_at: null, updated_at: null, analysis_sync_status: "not_analyzed",
  analysis_sync_error: null, analysis_sync_can_resume: false, analysis_sync_has_saved_analysis: false,
  analysis_sync_resume_block_reason: null, derived_sync_status: index === 0 ? "failed" : "completed", derived_sync_error: null,
  derived_sync_can_resume: false, derived_sync_workflow_run_id: null, material_updates: [],
  material_sync: Object.fromEntries(["characters", "objects", "scenes", "content_index", "outline_and_foreshadowing"].map((key) => [key, "completed"])),
}));
const responses = {
  get_current_user: {
    id: 1, username: "写作者", email: "writer@example.test", role: "user", is_active: true,
    page_cli_enabled: false, avatar_url: null, created_at: null, updated_at: null,
    last_login_at: null, oauth_provider: null, oauth_subject: null,
  },
  list_projects: { items: projects, page: 1, page_size: 20, total: projects.length },
  get_project: project,
  list_chapters: chapters,
  list_chapter_outlines: [],
  list_my_project_invites: [],
  list_themes: [],
  get_theme_preference: { appearance: "light", user_id: 1 },
  get_blueprint: null,
  list_profiles: [],
  active: [],
  list_llm_config: [],
  list_model_provider_catalog: {
    llm: [{ id: "deepseek", display_name: "DeepSeek", default_base_url: "https://api.deepseek.com/v1" }],
    embedding: [], image: [], video: [],
  },
};

const button = (label) => $(`button[aria-label="${label}"]`);
const commands = () => browser.execute(() => window.__uiAudit.commands);
const draws = () => browser.execute(() => window.__uiAudit.draws);

describe("UI presentation and workspace continuity", () => {
  before(async () => {
    await browser.addInitScript((encodedFixtures) => {
      const fixtures = JSON.parse(decodeURIComponent(encodedFixtures));
      localStorage.setItem("a4n:current_user_id", "1");
      localStorage.setItem("a4n:current_access_token", "ui-test-fixture");
      localStorage.setItem("a4n:settings:locale", "zh-CN");
      window.__uiAudit = { commands: [], unexpected: [], errors: [], draws: { attached: 0, detached: 0 } };
      window.addEventListener("error", (event) => window.__uiAudit.errors.push(event.message));
      window.addEventListener("unhandledrejection", (event) => window.__uiAudit.errors.push(String(event.reason)));
      const fetch = window.fetch.bind(window);
      window.fetch = async (input, options) => {
        const url = new URL(typeof input === "string" ? input : input.url, location.href);
        if (!url.pathname.startsWith("/api/")) return fetch(input, options);
        const command = url.pathname.split("/").at(-1);
        window.__uiAudit.commands.push(command);
        if (!(command in fixtures)) {
          window.__uiAudit.unexpected.push(command);
          return new Response(JSON.stringify({ message: `Missing UI fixture: ${command}` }), { status: 501 });
        }
        return new Response(JSON.stringify(fixtures[command]), { headers: { "Content-Type": "application/json" } });
      };
      const clearRect = CanvasRenderingContext2D.prototype.clearRect;
      CanvasRenderingContext2D.prototype.clearRect = function (...args) {
        if (this.canvas.width === 520 && this.canvas.height === 240) {
          window.__uiAudit.draws[this.canvas.isConnected ? "attached" : "detached"]++;
        }
        return clearRect.apply(this, args);
      };
    }, encodeURIComponent(JSON.stringify(responses)));
  });

  afterEach(async () => {
    const { unexpected, errors } = await browser.execute(() => window.__uiAudit);
    assert.deepEqual(unexpected, []);
    assert.deepEqual(errors, []);
  });

  it("opens menus by keyboard without opening the project, and keeps disabled actions inert", async () => {
    await browser.url("/");
    const trigger = await button("更多操作");
    await trigger.waitForDisplayed();
    await trigger.execute((element) => element.focus());
    await browser.keys("Enter");
    await $('[role="menu"]').waitForDisplayed();
    assert.equal(new URL(await browser.getUrl()).pathname, "/");
    await browser.keys("Escape");
    assert.equal(await trigger.isFocused(), true);

    const protectedTrigger = await $$('.a4n-project-row button[aria-label="更多操作"]')[2];
    await protectedTrigger.click();
    await browser.waitUntil(async () => await browser.execute(() => Boolean(document.activeElement?.closest('[role="menu"]'))));
    await browser.keys("End");
    await browser.waitUntil(async () => await browser.execute(() => document.activeElement?.getAttribute("aria-disabled") === "true"));
    await browser.keys("Enter");
    assert.equal(await $('[role="menu"]').isDisplayed(), true);
    assert.equal((await commands()).includes("delete_project"), false);
    await browser.keys("Tab");
    await $('[role="menu"]').waitForExist({ reverse: true });

    const entry = await $('button[aria-label="山海之间 · 进入写作台"]');
    await entry.execute((element) => element.focus());
    await entry.click({ button: "right" });
    await $('[role="menu"]').waitForDisplayed();
    await browser.keys("Escape");
    await $('[role="menu"]').waitForExist({ reverse: true });
    assert.equal(await entry.isFocused(), true);
  });

  it("preserves the workspace across chapter changes and keeps a new unsaved chapter selected", async () => {
    await browser.url("/write/1?chapter=1");
    const editor = await $('textarea[aria-label="章节正文"]');
    await editor.waitForDisplayed();
    await browser.waitUntil(async () => (await editor.getValue()).includes("旧书店"));
    assert.equal((await $('[role="alert"]').getText()).match(/资料同步未完成/g)?.length, 1);
    const before = await commands();
    await browser.execute(() => { window.__initialWorkspace = document.querySelector(".a4n-route-stage"); });
    const search = await $('input[placeholder*="搜索章节"]');
    await search.setValue("旧书店");
    await button("下一章").click();
    await browser.waitUntil(async () => (await editor.getValue()).includes("未寄出的信"));
    assert.equal(await browser.execute(() => window.__initialWorkspace === document.querySelector(".a4n-route-stage")), true);
    assert.equal(await search.getValue(), "旧书店");
    assert.deepEqual(await commands(), before);

    await browser.execute(() => {
      history.pushState(history.state, "", "?chapter=1");
      window.dispatchEvent(new PopStateEvent("popstate"));
    });
    await browser.waitUntil(async () => (await editor.getValue()).includes("旧书店"));
    assert.deepEqual(await commands(), before);

    await button("新建章节").click();
    await $('//button[normalize-space()="进入章节"]').click();
    await $('[role="dialog"]').waitForExist({ reverse: true });
    await browser.waitUntil(async () => (await $("h1").getText()).includes("第 3 章"));
    assert.equal(new URL(await browser.getUrl()).searchParams.get("chapter"), "3");
    assert.deepEqual(await commands(), before);
    await editor.setValue("第三章尚未保存的正文");
    assert.equal(await editor.getValue(), "第三章尚未保存的正文");
    await button("上一章").click();
    await $('[role="dialog"]').waitForDisplayed();
    await browser.keys("Escape");
    await $('[role="dialog"]').waitForExist({ reverse: true });
    assert.equal(new URL(await browser.getUrl()).searchParams.get("chapter"), "3");
    assert.equal(await editor.getValue(), "第三章尚未保存的正文");
    await button("上一章").click();
    await $('//button[normalize-space()="继续"]').click();
    await browser.waitUntil(async () => (await editor.getValue()).includes("未寄出的信"));

    await browser.url("/write/1?chapter=3");
    await $('//button[normalize-space()="恢复草稿"]').click();
    await $('[role="dialog"]').waitForExist({ reverse: true });
    const restoredEditor = await $('textarea[aria-label="章节正文"]');
    await browser.waitUntil(async () => (await restoredEditor.getValue()) === "第三章尚未保存的正文");
    assert.equal(new URL(await browser.getUrl()).searchParams.get("chapter"), "3");
    await button("上一章").click();
    await $('//button[normalize-space()="继续"]').click();
    await browser.waitUntil(async () => (await restoredEditor.getValue()).includes("未寄出的信"));
  });

  it("rejects invalid chapter query values without inventing a chapter", async () => {
    for (const value of ["0", "1.5", "2147483648"]) {
      await browser.url(`/write/1?chapter=${value}`);
      await browser.waitUntil(async () => (await $("h1").getText()).includes("第 2 章"));
      assert.equal(new URL(await browser.getUrl()).searchParams.get("chapter"), "2");
    }
  });

  it("stops idle logo rendering, resumes on pointer movement, and stops after leaving", async () => {
    await browser.url("/");
    const canvas = await $("canvas[role=img]");
    await canvas.waitForDisplayed();
    let previous = -1;
    let steady = 0;
    await browser.waitUntil(async () => {
      const current = (await draws()).attached;
      steady = current > 0 && current === previous ? steady + 1 : 0;
      previous = current;
      return steady >= 2;
    }, { timeout: 15_000, interval: 300 });
    const settled = await draws();
    await canvas.moveTo();
    await browser.waitUntil(async () => (await draws()).attached > settled.attached);
    await $('//button[normalize-space()="设置"]').click();
    await $('section[aria-label="LLM 服务"]').waitForDisplayed();
    await browser.pause(350);
    assert.equal((await draws()).detached, 0);
  });

  it("keeps project and settings controls inside narrow viewports", async () => {
    for (const width of [390, 1024]) {
      await browser.setWindowSize(width, 900);
      for (const route of ["/", "/settings"]) {
        await browser.url(route);
        await $("h1").waitForExist();
        await browser.waitUntil(async () => (await commands()).includes(route === "/" ? "list_projects" : "list_llm_config"));
        assert.equal(await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth), 0);
        if (route === "/") {
          const kind = await $('//label/span[normalize-space()="项目类型"]');
          await kind.waitForDisplayed();
          assert.equal(await kind.isDisplayed(), true);
          await button("项目类型").click();
          await $('[role="listbox"]').waitForDisplayed();
          const bounds = await browser.execute(() => {
            const rect = document.querySelector('[role="listbox"]').getBoundingClientRect();
            return { left: rect.left, right: rect.right, width: innerWidth };
          });
          assert.ok(bounds.left >= 0 && bounds.right <= bounds.width);
          await browser.keys("Escape");
        }
      }
    }
  });
});
