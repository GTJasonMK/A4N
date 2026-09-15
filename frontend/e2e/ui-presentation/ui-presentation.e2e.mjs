import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

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
}, {
  ...project, id: 4, title: "只读协作项目", my_role: "viewer", owner_username: "另一位作者",
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
const agentSnapshots = ["主 Agent", "分镜 Agent"].map((title, index) => ({
  protocol_version: 1,
  thread: {
    id: `thread-${index}`, title, kind: index === 0 ? "main" : "subagent", status: "active",
    capability: index === 0 ? null : "storyboard", bound_resource: null, context_summary: null,
    default_model_config_id: null, created_at: "2026-09-11T12:00:00Z", updated_at: "2026-09-11T12:00:00Z",
    next_event_sequence: 1, next_turn_sequence: 1, owner_user_id: 1, protocol_version: 1, revision: 1,
    scope: { allowed_capabilities: [], allowed_tools: [], chapter_id: null, project_id: null, resource_id: null, resource_kind: null },
  },
  approvals: [], delegations: [], events: [], executions: [], inbox: [], memories: [], turns: [],
  runs: [{
    id: `run-${index}`, thread_id: `thread-${index}`, turn_id: `turn-${index}`, attempt: 1,
    status: "running", waiting_reason: null, lease_owner: null, lease_expires_at: null,
    budget: {
      max_children: null, max_cost_micros: null, max_delegation_depth: null,
      max_duration_seconds: null, max_model_calls: null, max_parallel_tools: null,
      max_steps: null, max_tokens: null, max_tool_calls: null,
    },
    steps_used: 1, model_calls_used: 1, tool_calls_used: 0, tokens_used: 500,
    context_input_tokens: 420, context_capacity_tokens: 1000,
    cost_micros: null, error: null, created_at: "2026-09-11T12:00:00Z", started_at: "2026-09-15T12:00:00Z", updated_at: "2026-09-15T12:00:00Z",
  }],
  messages: [{
    id: `reply-${index}`, thread_id: `thread-${index}`, role: "assistant", status: "complete",
    content: index === 0 ? Array.from({ length: 18 }, (_, paragraph) => `第 ${paragraph + 1} 段审阅：人物的行动应与前文保持一致。保留旧书店和信件的线索，让转场来自角色的选择。`).join("\n\n") : "分镜方案已准备好，可以继续讨论角色的行动。",
    created_at: "2026-09-11T12:00:00Z", updated_at: null, metadata: null, run_id: null,
    source_id: null, source_kind: null, turn_id: null,
  }],
}));
const responses = {
  agentSnapshots,
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
  list_themes: [{
    id: 1, user_id: 1, name: "书房主题", mode: "light", is_active: false, is_system_default: false,
    document: { version: 1, backgrounds: {}, tokens: { "--color-primary": "151 72 29", "--color-bg-primary": "241 238 232", "--color-bg-secondary": "255 252 245", "--color-text-primary": "44 24 16", "--color-border": "205 194 181" } },
    resources: [],
  }],
  get_theme_preference: { appearance: "light", user_id: 1 },
  prompt_list: [
    {
      name: "story_prompt", title: "写作提示词", description: "写作流程", category: "writing",
      content: "保留人物行动的因果。", dependencies: [], is_modified: false,
      path: "prompts/writing/story.md", tags: [], used_by: [], version: "1",
    },
    {
      name: "image_prompt", title: "画面提示词", description: "画面生成", category: "media",
      content: "保持画面构图稳定。", dependencies: [], is_modified: false,
      path: "prompts/media/image.md", tags: [], used_by: [], version: "1",
    },
  ],
  prompt_list_categories: [
    { key: "writing", label: "写作", description: "写作相关提示词", order: 1 },
    { key: "media", label: "素材", description: "素材相关提示词", order: 2 },
  ],
  get_blueprint: null,
  list_profiles: [],
  active: [],
  list_llm_config: [],
  list_model_provider_catalog: {
    llm: [
      { id: "deepseek", display_name: "DeepSeek", default_base_url: "https://api.deepseek.com/v1" },
      { id: "openai", display_name: "OpenAI", default_base_url: "https://api.openai.com/v1" },
    ],
    embedding: [], image: [], video: [],
  },
};

const button = (label) => $(`button[aria-label="${label}"]`);
const commands = () => browser.execute(() => window.__uiAudit.commands);
const draws = () => browser.execute(() => window.__uiAudit.draws);
const fieldInput = (name) => $(`//label[./span[contains(.,"${name}")]]//input`);
const replaceInput = async (input, value) => {
  await input.click();
  await browser.keys(["Control", "a"]);
  await browser.keys(value || "Backspace");
  assert.equal(await input.getValue(), value);
};
const openNewLlmConfig = async () => {
  await browser.setWindowSize(1440, 900);
  await browser.url("/settings#llm");
  await $('//button[contains(.,"新增 LLM 配置")]').click();
  await replaceInput(await fieldInput("配置名称"), "配置 A");
  await replaceInput(await $('input[role="combobox"]'), "");
};
const finishModelDiscovery = (index, id) => browser.execute((requestIndex, modelId) => {
  window.__uiAudit.modelRequests[requestIndex].resolve(new Response(
    JSON.stringify({ models: modelId ? [{ id: modelId, name: modelId, owned_by: null }] : [] }),
    { headers: { "Content-Type": "application/json" } },
  ));
}, index, id);
const finishAgentSend = (error = null) => browser.execute((errorMessage) => {
  const { resolve, input } = window.__uiAudit.agentMessages[0];
  const body = errorMessage ? {
    code: "provider_unavailable", message: errorMessage, retryable: true, trace_id: null, details: {},
  } : {
    created_run: true, message_id: "message-1", run_id: "run-1", turn_id: "turn-1",
    thread_id: input.thread_id, status: "queued",
  };
  resolve(new Response(JSON.stringify(body), { status: errorMessage ? 503 : 200, headers: { "Content-Type": "application/json" } }));
}, error);
const agentMessageCount = () => browser.execute(() => window.__uiAudit.agentMessages.length);
const selectAgentThread = (title) => $(`//button[contains(@class,"a4n-sidebar-item") and .//span[text()="${title}"]]`).click();

describe("UI presentation and workspace continuity", () => {
  before(async () => {
    await browser.addInitScript((encodedFixtures) => {
      const fixtures = JSON.parse(decodeURIComponent(encodedFixtures));
      localStorage.setItem("a4n:current_user_id", "1");
      localStorage.setItem("a4n:current_access_token", "ui-test-fixture");
      localStorage.setItem("a4n:settings:locale", "zh-CN");
      window.__uiAudit = { commands: [], unexpected: [], errors: [], modelRequests: [], agentMessages: [], draws: { attached: 0, detached: 0 } };
      window.addEventListener("error", (event) => window.__uiAudit.errors.push(event.message));
      window.addEventListener("unhandledrejection", (event) => window.__uiAudit.errors.push(String(event.reason)));
      const fetch = window.fetch.bind(window);
      window.fetch = async (input, options) => {
        const url = new URL(typeof input === "string" ? input : input.url, location.href);
        if (!url.pathname.startsWith("/api/")) return fetch(input, options);
        const command = url.pathname.split("/").at(-1);
        window.__uiAudit.commands.push(command);
        if (url.pathname === "/api/agent-threads") {
          return new Response(JSON.stringify(fixtures.agentSnapshots.map((item) => item.thread)), { headers: { "Content-Type": "application/json" } });
        }
        const agentPath = url.pathname.match(/^\/api\/agent-threads\/(thread-\d)(?:\/(messages|events))?$/);
        if (agentPath) {
          const snapshot = fixtures.agentSnapshots.find((item) => item.thread.id === agentPath[1]);
          if (!snapshot) throw new Error(`Missing Agent fixture: ${agentPath[1]}`);
          if (agentPath[2] === "messages" && options.method === "POST") {
            return new Promise((resolve) => window.__uiAudit.agentMessages.push({ input: JSON.parse(options.body), resolve }));
          }
          if (agentPath[2] === "events") {
            return new Response(new ReadableStream({
              start(controller) {
                if (options.signal.aborted) controller.close();
                else options.signal.addEventListener("abort", () => controller.close(), { once: true });
              },
            }), { headers: { "Content-Type": "text/event-stream" } });
          }
          if (!agentPath[2]) return new Response(JSON.stringify(snapshot), { headers: { "Content-Type": "application/json" } });
          throw new Error(`Unexpected Agent fixture request: ${options.method} ${url.pathname}`);
        }
        if (command === "get_project") {
          const projectId = JSON.parse(options.body).projectId;
          const record = fixtures.list_projects.items.find((item) => item.id === projectId);
          if (!record) throw new Error(`Missing project fixture: ${projectId}`);
          return new Response(JSON.stringify(record), { headers: { "Content-Type": "application/json" } });
        }
        if (command === "update_theme") {
          const { themeId, input } = JSON.parse(options.body);
          const record = fixtures.list_themes.find((item) => item.id === themeId);
          Object.assign(record, input);
          return new Response(JSON.stringify(record), { headers: { "Content-Type": "application/json" } });
        }
        if (command === "detect_llm_config_models") {
          return new Promise((resolve) => {
            window.__uiAudit.modelRequests.push({ resolve });
          });
        }
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

  beforeEach(async () => {
    // A same-fragment navigation keeps the previous document and its draft.
    await browser.url("/");
    await browser.setWindowSize(1440, 900);
  });

  afterEach(async function () {
    const directory = fileURLToPath(new URL("../../../.dev-runtime/ui-details/", import.meta.url));
    await mkdir(directory, { recursive: true });
    const name = this.currentTest.title.replace(/[^a-z0-9]+/gi, "-").slice(0, 100);
    await browser.saveScreenshot(`${directory}/${process.env.A4N_UI_AUDIT_PHASE ?? "current"}-${name}.png`);
    const { unexpected, errors } = await browser.execute(() => ({ unexpected: window.__uiAudit.unexpected, errors: window.__uiAudit.errors }));
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

  it("bounds logo rendering during interaction and stops when the pointer rests", async () => {
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
    const location = await canvas.getLocation();
    const size = await canvas.getSize();
    const center = { x: Math.round(location.x + size.width / 2), y: Math.round(location.y + size.height / 2) };
    const movement = browser.action("pointer").move(center);
    for (let step = 0; step < 16; step++) {
      movement.move({ x: center.x + (step % 2 ? 40 : -40), y: center.y, duration: 150 });
    }
    await movement.perform();
    await browser.waitUntil(async () => (await draws()).attached > settled.attached);
    await canvas.moveTo();
    await browser.pause(3_000);
    const hovering = await draws();
    await browser.pause(500);
    assert.equal((await draws()).attached, hovering.attached, "A stationary pointer must allow the logo to stop rendering");
    for (const width of [1024, 390]) {
      await browser.setWindowSize(width, 900);
      assert.equal(
        await browser.execute(() => document.documentElement.scrollWidth - document.documentElement.clientWidth),
        0,
        `horizontal overflow at ${width}px`,
      );
    }
    await browser.setWindowSize(1440, 900);
    await $('//button[normalize-space()="设置"]').click();
    await $(".a4n-settings-layout").waitForDisplayed();
    await browser.pause(350);
    assert.equal((await draws()).detached, 0);
  });

  it("preserves newer form edits when model discovery completes", async () => {
    await openNewLlmConfig();
    const detect = await $('button[title="检测模型列表"]');
    await detect.click();
    await browser.waitUntil(async () => (await browser.execute(() => window.__uiAudit.modelRequests.length)) === 1);
    await replaceInput(await fieldInput("配置名称"), "更新后的配置名称");
    await finishModelDiscovery(0, "discovered-model");
    await detect.waitForEnabled();
    assert.equal(await fieldInput("配置名称").getValue(), "更新后的配置名称");
    assert.equal(await $('input[role="combobox"]').getValue(), "discovered-model");
  });

  it("isolates model discovery when the provider changes", async () => {
    await openNewLlmConfig();
    const detect = await $('button[title="检测模型列表"]');
    await detect.click();
    await browser.waitUntil(async () => (await browser.execute(() => window.__uiAudit.modelRequests.length)) === 1);
    await $('button[aria-label="厂商预设"]').click();
    await $('//div[@role="option" and normalize-space()="OpenAI"]').click();
    await detect.waitForEnabled();
    await detect.click();
    await browser.waitUntil(async () => (await browser.execute(() => window.__uiAudit.modelRequests.length)) === 2);
    await finishModelDiscovery(0, "old-provider-model");
    await finishModelDiscovery(1, "new-provider-model");
    await detect.waitForEnabled();
    assert.match(await $('button[aria-label="厂商预设"]').getText(), /OpenAI/);
    assert.equal(await $('input[role="combobox"]').getValue(), "new-provider-model");
  });

  it("explains an empty model catalog without replacing the form", async () => {
    await openNewLlmConfig();
    const detect = await $('button[title="检测模型列表"]');
    await detect.click();
    await browser.waitUntil(async () => (await browser.execute(() => window.__uiAudit.modelRequests.length)) === 1);
    await finishModelDiscovery(0, null);
    await detect.waitForEnabled();
    await $('//*[contains(text(),"未发现可用模型")]').waitForDisplayed();
    assert.equal(await $('input[role="combobox"]').getValue(), "");
  });

  it("opens readonly projects in a readable view without offering writing actions", async () => {
    await browser.url("/");
    const entry = await $('button[aria-label="只读协作项目 · 进入项目详情"]');
    await entry.waitForDisplayed();
    const card = await entry.$('..');
    assert.equal(await card.$('button[aria-label="进入写作台"]').isExisting(), false);
    assert.equal(await card.$('button[aria-label="更多操作"]').isExisting(), false);
    await entry.click();
    await browser.waitUntil(async () => new URL(await browser.getUrl()).pathname === "/novel/4");
    await $('//h1[normalize-space()="只读协作项目"]').waitForDisplayed();
    await $('//*[text()="还没有故事蓝图"]').waitForDisplayed();
    assert.equal(await $('//*[text()="从现有资料归纳、直接录入或从灵感对话开始"]').isExisting(), false);
    assert.equal((await commands()).includes("update_chapter_content"), false);
  });

  it("does not save a theme name when Enter confirms IME composition", async () => {
    await browser.url("/settings#theme");
    await $('button[title="重命名"]').click();
    const name = await $('input[aria-label="重命名"]');
    await replaceInput(name, "书房主题修改");
    await name.execute((element) => element.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", isComposing: true, bubbles: true, cancelable: true })));
    assert.equal((await commands()).includes("update_theme"), false);
    assert.equal(await name.isExisting(), true);
    await $('//button[normalize-space()="保存"]').click();
    await browser.waitUntil(async () => (await commands()).includes("update_theme"));
    await name.waitForExist({ reverse: true });
    assert.equal(await $('//button[normalize-space()="书房主题修改"]').isDisplayed(), true);
    assert.deepEqual(await browser.execute(() => {
      const toast = document.querySelector(".a4n-toast").getBoundingClientRect();
      return [...document.querySelectorAll(".a4n-settings-layout button, .a4n-settings-layout input")].filter((element) => {
        const rect = element.getBoundingClientRect();
        return rect.width > 1 && rect.height > 1 && toast.left < rect.right && toast.right > rect.left && toast.top < rect.bottom && toast.bottom > rect.top;
      }).map((element) => element.getAttribute("aria-label") || element.textContent);
    }), [], "A saved notification must leave settings controls available");
  });

  it("honors settings deep links and keeps the page stable when switching categories", async () => {
    await browser.execute(() => localStorage.setItem("a4n:settings:tab", "llm"));
    await browser.url("/settings#theme");
    await $('[aria-label="外观偏好"]').waitForDisplayed();
    assert.equal(new URL(await browser.getUrl()).hash, "#theme");
    await browser.execute(() => {
      window.__settingsStage = document.querySelector(".a4n-route-stage");
      window.__settingsDirection = window.__settingsStage.dataset.routeDirection;
    });
    await $('//nav//button[contains(.,"LLM")]').click();
    await $('//button[contains(.,"新增 LLM 配置")]').waitForDisplayed();
    assert.equal(new URL(await browser.getUrl()).hash, "#llm");
    assert.equal(await browser.execute(() => window.__settingsStage === document.querySelector(".a4n-route-stage") && window.__settingsDirection === window.__settingsStage.dataset.routeDirection), true);
  });

  it("protects unsaved settings drafts when switching categories", async () => {
    await openNewLlmConfig();
    await $('//nav//button[contains(.,"主题")]').click();
    await $('[role="dialog"]').waitForDisplayed();
    assert.equal(new URL(await browser.getUrl()).hash, "#llm");
    assert.equal(await fieldInput("配置名称").getValue(), "配置 A");

    await browser.keys("Escape");
    await $('[role="dialog"]').waitForExist({ reverse: true });
    assert.equal(new URL(await browser.getUrl()).hash, "#llm");

    await $('//nav//button[contains(.,"主题")]').click();
    await $('//button[normalize-space()="放弃草稿"]').click();
    await $('[aria-label="外观偏好"]').waitForDisplayed();
    assert.equal(new URL(await browser.getUrl()).hash, "#theme");
  });

  it("protects theme editor drafts when closing the editor", async () => {
    await browser.url("/settings#theme");
    await $('button[aria-label="编辑主题"]').click();
    const editor = await $('[role="dialog"]');
    await editor.waitForDisplayed();
    const name = await editor.$("input");
    await replaceInput(name, "书房主题草稿");

    const closeEditor = async () => $('//div[@role="dialog"]//button[@aria-label="关闭"]').click();
    await closeEditor();
    await $('[role="dialog"]').waitForDisplayed();
    assert.equal(new URL(await browser.getUrl()).hash, "#theme");
    assert.equal(await name.getValue(), "书房主题草稿");
    await browser.keys("Escape");
    await $('//button[normalize-space()="放弃草稿"]').waitForExist({ reverse: true });

    await closeEditor();
    await $('//button[normalize-space()="放弃草稿"]').click();
    await editor.waitForExist({ reverse: true });
    assert.equal(new URL(await browser.getUrl()).hash, "#theme");
  });

  it("protects theme action and rename drafts when closing", async () => {
    await browser.url("/settings#theme");
    await browser.setWindowSize(390, 900);
    await browser.setWindowSize(1440, 900);
    await $('//button[contains(.,"创建主题")]').click();
    const createDialog = await $('[role="dialog"]');
    await createDialog.waitForDisplayed();
    const name = await createDialog.$("input");
    await replaceInput(name, "夜读主题草稿");

    await createDialog.$('button[aria-label="关闭"]').click();
    await $('//button[normalize-space()="放弃草稿"]').waitForDisplayed();
    await browser.keys("Escape");
    assert.equal(await name.getValue(), "夜读主题草稿");
    await createDialog.$('button[aria-label="关闭"]').click();
    await $('//button[normalize-space()="放弃草稿"]').click();
    await createDialog.waitForExist({ reverse: true });

    await $('//button[normalize-space()="书房主题"]').click();
    const renameInput = await $('input[aria-label="重命名"]');
    await replaceInput(renameInput, "书房主题草稿");
    await browser.keys("Escape");
    await $('//button[normalize-space()="放弃草稿"]').waitForDisplayed();
    await browser.keys("Escape");
    assert.equal(await renameInput.getValue(), "书房主题草稿");
    await browser.keys("Escape");
    await $('//button[normalize-space()="放弃草稿"]').click();
    await renameInput.waitForExist({ reverse: true });
  });

  it("protects a theme mode draft even when its name is empty", async () => {
    await browser.url("/settings#theme");
    await $('//button[contains(.,"创建主题")]').click();
    const createDialog = await $('[role="dialog"]');
    await createDialog.waitForDisplayed();
    await createDialog.$('//button[normalize-space()="暗色"]').click();

    await createDialog.$('button[aria-label="关闭"]').click();
    await $('//button[normalize-space()="放弃草稿"]').waitForDisplayed();
    await browser.keys("Escape");
    assert.equal(await createDialog.$('//button[normalize-space()="暗色"]').getAttribute("data-active"), "1");

    await createDialog.$('button[aria-label="关闭"]').click();
    await $('//button[normalize-space()="放弃草稿"]').click();
    await createDialog.waitForExist({ reverse: true });

    await $('//button[contains(.,"创建主题")]').click();
    const freshDialog = await $('[role="dialog"]');
    await freshDialog.waitForDisplayed();
    assert.equal(await freshDialog.$('//button[normalize-space()="亮色"]').getAttribute("data-active"), "1");
    await freshDialog.$('button[aria-label="关闭"]').click();
    await freshDialog.waitForExist({ reverse: true });
  });

  it("protects prompt drafts when switching categories", async () => {
    await browser.url("/settings#prompt");
    await browser.setWindowSize(390, 900);
    await browser.setWindowSize(1440, 900);
    await $('//button[contains(.,"写作提示词")]').click();
    const editor = await $("textarea");
    await editor.waitForDisplayed();
    await replaceInput(editor, "写作提示词草稿");

    await $('//button[contains(@class,"pyqt-tab") and normalize-space()="素材"]').click();
    await $('//button[normalize-space()="放弃草稿"]').waitForDisplayed();
    assert.deepEqual(
      await browser.execute(() => [...document.querySelectorAll(".tab-bar button[data-active=\"1\"]")].map((item) => item.textContent?.trim())),
      ["全部"],
    );
    await browser.keys("Escape");

    await $('//button[contains(@class,"pyqt-tab") and normalize-space()="素材"]').click();
    await $('//button[normalize-space()="放弃草稿"]').click();
    await browser.waitUntil(async () => {
      const active = await browser.execute(() => [...document.querySelectorAll(".tab-bar button[data-active=\"1\"]")].map((item) => item.textContent?.trim()));
      return active.join() === "素材" && (await $("textarea").getValue()) === "保持画面构图稳定。";
    });
    assert.equal(await $("textarea").getValue(), "保持画面构图稳定。");
  });

  it("keeps unsent Agent drafts with their thread", async () => {
    await browser.url("/agents");
    const input = await $('textarea[aria-label="Agent 消息"]');
    await input.waitForDisplayed();
    await input.setValue("主 Agent 的未发送消息");
    await selectAgentThread("分镜 Agent");
    assert.equal(await input.getValue(), "");
    await input.setValue("分镜 Agent 的未发送消息");
    await selectAgentThread("主 Agent");
    assert.equal(await input.getValue(), "主 Agent 的未发送消息");
    await selectAgentThread("分镜 Agent");
    assert.equal(await input.getValue(), "分镜 Agent 的未发送消息");
  });

  it("shows the latest fitted Agent context usage", async () => {
    await browser.url("/agents");
    const meter = await $(".agent-context-meter");
    await meter.waitForDisplayed();
    assert.match(await meter.getText(), /上下文/);
    assert.match(await meter.getText(), /42%/);
    assert.equal(await $(".agent-monitor-activity-pane").isExisting(), false);
    assert.equal(await $(".agent-monitor-activity-toggle").getAttribute("aria-pressed"), "false");
  });

  it("keeps a late Agent send from replacing the selected thread", async () => {
    await browser.url("/agents");
    const input = await $('textarea[aria-label="Agent 消息"]');
    await input.waitForDisplayed();
    await input.setValue("请继续主线程任务");
    await button("发送消息").click();
    await browser.waitUntil(async () => (await agentMessageCount()) === 1);
    const mainThread = await $('//button[contains(@class,"a4n-sidebar-item") and .//span[text()="主 Agent"]]');
    assert.equal(await mainThread.getAttribute("aria-busy"), "true");
    await selectAgentThread("分镜 Agent");
    await $('//h2[text()="分镜 Agent"]').waitForDisplayed();
    await input.waitForEnabled();
    await input.setValue("分镜线程的新草稿");
    await finishAgentSend();
    await browser.waitUntil(async () => (await mainThread.getAttribute("aria-busy")) !== "true");
    assert.equal(await $('//h2[text()="分镜 Agent"]').isDisplayed(), true);
    assert.equal(await $('//h2[text()="主 Agent"]').isExisting(), false);
    assert.equal(await input.getValue(), "分镜线程的新草稿");
    await selectAgentThread("主 Agent");
    assert.equal(await input.getValue(), "");
  });

  it("does not send an Agent message while confirming IME input", async () => {
    await browser.url("/agents");
    const input = await $('textarea[aria-label="Agent 消息"]');
    await input.waitForDisplayed();
    await input.setValue("正在输入的中文");
    await input.execute((element) => element.dispatchEvent(new KeyboardEvent("keydown", {
      key: "Enter", ctrlKey: true, isComposing: true, bubbles: true, cancelable: true,
    })));
    assert.equal(await agentMessageCount(), 0);
    await browser.keys(["Control", "Enter"]);
    await browser.waitUntil(async () => (await agentMessageCount()) === 1);
    await browser.waitUntil(async () => browser.execute(() => (
      document.querySelector('[data-agent-message-pending="true"]')?.textContent?.includes("正在输入的中文") ?? false
    )));
    assert.equal(await browser.execute(() => (
      document.querySelector('[data-agent-message-pending="true"]')?.textContent?.match(/正在输入的中文/g)?.length ?? 0
    )), 1);
    await finishAgentSend();
    await input.waitForEnabled();
    assert.equal(await input.getValue(), "");
  });

  it("preserves the Agent draft and shows the actual send failure", async () => {
    await browser.url("/agents");
    const input = await $('textarea[aria-label="Agent 消息"]');
    await input.waitForDisplayed();
    await input.setValue("发送失败时仍需保留的文字");
    await button("发送消息").click();
    await browser.waitUntil(async () => (await agentMessageCount()) === 1);
    await finishAgentSend("模型服务暂时无法连接");
    await input.waitForEnabled();
    assert.equal(await input.getValue(), "发送失败时仍需保留的文字");
    const alert = await $('.a4n-toast[role="alert"]');
    await alert.waitForDisplayed();
    assert.match(await alert.getText(), /模型服务暂时无法连接/);
  });

  it("keeps narrow workspace controls readable and message history keyboard scrollable", async () => {
    for (const width of [390, 1024]) {
      await browser.setWindowSize(width, 900);
      for (const route of ["/", "/settings#llm", "/agents"]) {
        await browser.url(route);
        await $("h1").waitForExist();
        const expectedCommand = route === "/" ? "list_projects" : route === "/agents" ? "agent-threads" : "list_llm_config";
        await browser.waitUntil(async () => (await commands()).includes(expectedCommand));
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
        if (route === "/agents") {
          const log = await $('[role="log"]');
          await log.waitForDisplayed();
          const transcriptPresentation = await browser.execute(() => {
            const row = document.querySelector('[data-agent-message-role="assistant"]');
            const stack = row?.querySelector(':scope > .agent-message-stack');
            const style = stack ? getComputedStyle(stack) : null;
            return {
              rowClass: row?.className ?? null,
              stackClass: stack?.className ?? null,
              border: style?.borderTopWidth ?? null,
              background: style?.backgroundColor ?? null,
              backToLatest: document.querySelector('[aria-label="回到最新"]') !== null,
            };
          });
          assert.match(String(transcriptPresentation.rowClass), /agent-message-row/);
          assert.match(String(transcriptPresentation.stackClass), /agent-message-stack/);
          assert.equal(transcriptPresentation.border, "0px");
          assert.equal(transcriptPresentation.backToLatest, false);
          assert.ok((await log.getSize()).height >= 180, "Message history needs enough room to read several lines");
          await browser.waitUntil(async () => (await log.execute((element) => element.scrollTop)) > 0, {
            timeout: 5_000,
            interval: 50,
            timeoutMsg: "Message history should follow the latest message on load",
          });
          await log.execute((element) => element.focus());
          await browser.keys("End");
          await browser.waitUntil(async () => (await log.execute((element) => element.scrollTop)) > 0);
        }
      }
    }
  });
});
