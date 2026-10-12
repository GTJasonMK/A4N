// Shared, explicit API fixtures for the real application. Unknown requests fail with 501.
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
export const uiFixtures = {
  projectLoadError: "项目暂时无法加载，请稍后重试",
  agentSnapshots,
  projectRecords: projects,
  get_current_user: {
    id: 1, username: "写作者", email: "writer@example.test", role: "user", is_active: true,
    page_cli_enabled: false, avatar_url: null, created_at: null, updated_at: null,
    last_login_at: null, oauth_provider: null, oauth_subject: null,
  },
  get_package_preferences: { host_scope: "ui-fixture-host", hidden_package_ids: [] },
  get_project_fact_feedback: {
    project_id: 1, interval_chapters: 5, consolidated_chapters: [], pending_chapters: [],
    legacy_chapters: [1, 2], last_consolidated_at: null, workflow_kind: null,
    workflow_run_id: null, workflow_status: null, can_resume: false, error: null, changes: [],
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
      { id: "deepseek", display_name: "DeepSeek", default_base_url: "https://api.deepseek.com/v1", protocol: "openai_chat_completions" },
      { id: "openai", display_name: "OpenAI", default_base_url: "https://api.openai.com/v1", protocol: "openai_chat_completions" },
    ],
    embedding: [], image: [], video: [],
  },
};

let fixtureScript;
export async function installUiFixtures(browser, overrides = {}) {
    await fixtureScript?.remove();
    fixtureScript = await browser.addInitScript((encodedFixtures) => {
      const fixtures = JSON.parse(decodeURIComponent(encodedFixtures));
      const params = new URL(location.href).searchParams;
      const authenticated = !params.has("ui-auth");
      if (authenticated) {
        localStorage.setItem("a4n:current_user_id", "1");
        localStorage.setItem("a4n:current_access_token", "ui-test-fixture");
      } else {
        localStorage.removeItem("a4n:current_user_id");
        localStorage.removeItem("a4n:current_access_token");
      }
      localStorage.setItem("a4n:settings:locale", params.get("ui-locale") ?? "zh-CN");
      window.__uiAudit = { fixtures, projectState: params.get("ui-state"), commands: [], requests: [], deferred: [], pending: [], unexpected: [], errors: [], modelRequests: [], agentMessages: [], emitPageSessionNavigate: null, draws: { attached: 0, detached: 0 } };
      window.addEventListener("error", (event) => window.__uiAudit.errors.push(event.message));
      window.addEventListener("unhandledrejection", (event) => window.__uiAudit.errors.push(String(event.reason)));
      const fetch = window.fetch.bind(window);
      window.fetch = async (input, options) => {
        const url = new URL(typeof input === "string" ? input : input.url, location.href);
        if (!url.pathname.startsWith("/api/")) return fetch(input, options);
        const command = fixtures.commandByPath?.[url.pathname] ?? url.pathname.split("/").at(-1);
        window.__uiAudit.commands.push(command);
        const body = options?.body ? JSON.parse(options.body) : {};
        window.__uiAudit.requests.push({ command, body });
        if (window.__uiAudit.deferred.includes(command)) {
          return new Promise((resolve) => window.__uiAudit.pending.push({ command, body, resolve }));
        }
        if (url.pathname.startsWith("/api/ui-sessions") && fixtures.pageSessions) {
          const state = fixtures.pageSessions.createResponse;
          const method = options?.method ?? "GET";
          const touch = () => Object.assign(state.session, {
            last_seen_at: new Date().toISOString(), updated_at: new Date().toISOString(),
            expires_at: new Date(Date.now() + state.lease_seconds * 1000).toISOString(),
          });
          if (url.pathname === "/api/ui-sessions" && method === "POST") {
            Object.assign(state.session, { client: body.client, tab_id: body.tab_id });
            touch();
            return Response.json(state);
          }
          if (new Headers(options?.headers).get("X-UI-Session-Token") !== state.ui_session_token) {
            return Response.json({ message: "Missing fixture UI session token" }, { status: 401 });
          }
          if (url.pathname === "/api/ui-sessions/events" && method === "GET") {
            return new Response(new ReadableStream({ start(controller) {
              const emitPageSessionNavigate = (route) => {
                const session = fixtures.pageSessions.createResponse.session;
                session.route = route;
                session.context_revision += 1;
                touch();
                const event = { kind: "navigate", operation: null, reason: "fixture", route, session: { ...session } };
                controller.enqueue(new TextEncoder().encode(`event: page_session\ndata: ${JSON.stringify(event)}\n\n`));
              };
              const close = () => {
                controller.close();
                if (window.__uiAudit.emitPageSessionNavigate === emitPageSessionNavigate) {
                  window.__uiAudit.emitPageSessionNavigate = null;
                }
              };
              window.__uiAudit.emitPageSessionNavigate = emitPageSessionNavigate;
              if (options?.signal?.aborted) close();
              else options?.signal?.addEventListener("abort", close, { once: true });
            } }), { headers: { "Content-Type": "text/event-stream" } });
          }
          const prefix = `/api/ui-sessions/${state.session.ui_session_id}`;
          if (url.pathname === `${prefix}/authorize` && method === "POST") return Response.json(fixtures.pageSessions.authorizationResponse);
          if ((url.pathname === `${prefix}/context` && method === "PATCH")
            || (url.pathname === `${prefix}/heartbeat` && method === "POST")) {
            Object.assign(state.session, command === "context" ? body : body.context ?? {});
            touch();
            return Response.json(state.session);
          }
          if (url.pathname === `${prefix}/control/detach` && method === "POST") {
            state.session.status = "idle";
            state.session.agent_id = null;
            state.session.operation_id = null;
            state.session.mode = "page";
            return Response.json(state.session);
          }
        }
        if (command === "get_current_user" && !authenticated) {
          return new Response(JSON.stringify({ code: "unauthorized", message: "Not signed in" }), { status: 401 });
        }
        if (command === "list_projects") {
          const state = window.__uiAudit.projectState;
          if (state === "empty") return Response.json({ items: [], page: 1, page_size: 20, total: 0 });
          if (state === "error") return Response.json({ code: "unavailable", message: fixtures.projectLoadError, retryable: true, trace_id: null, details: {} }, { status: 503 });
          if (state === "loading") return new Promise((_resolve, reject) => {
            options?.signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")), { once: true });
          });
          const page = body.page ?? 1, pageSize = body.pageSize ?? 20;
          const access = body.accessScope ?? "all";
          if (!Number.isInteger(page) || page < 1 || !Number.isInteger(pageSize) || pageSize < 1
            || !["all", "owned", "collaborating"].includes(access)) throw new Error("Invalid project-list fixture query");
          const items = fixtures.list_projects.items.filter((project) =>
            (!body.projectKind || project.project_kind === body.projectKind)
            && (access === "all" || (access === "owned" ? project.my_role === "owner" : project.my_role !== "owner")),
          );
          return Response.json({ items: items.slice((page - 1) * pageSize, page * pageSize), page, page_size: pageSize, total: items.length });
        }
        if (url.pathname === "/api/agent-threads") {
          return new Response(JSON.stringify(fixtures.agentSnapshots.map((item) => item.thread)), { headers: { "Content-Type": "application/json" } });
        }
        const agentRunPath = url.pathname.match(/^\/api\/agent-runs\/([^/]+)$/);
        if (agentRunPath) {
          if ((options?.method ?? "GET") !== "GET") throw new Error(`Unexpected Agent run fixture method: ${options.method}`);
          const run = fixtures.agentSnapshots.flatMap((snapshot) => snapshot.runs).find((item) => item.id === agentRunPath[1]);
          if (!run) throw new Error(`Missing Agent run fixture: ${agentRunPath[1]}`);
          return Response.json(run);
        }
        if (url.pathname === "/api/generation-tasks/active" && fixtures.generationTasks) {
          const projectId = url.searchParams.get("project_id");
          const tasks = Object.values(fixtures.generationTasks).map((task) => task.summary);
          return Response.json(tasks.filter((task) => ["reserved", "running", "finalizing", "cancelling"].includes(task.handle.status)
            && (projectId == null || task.started.project_id === Number(projectId))));
        }
        if (command === "visual-plan-edits") {
          return new Response(new ReadableStream({
            start(controller) {
              if (options?.signal?.aborted) controller.close();
              else options?.signal?.addEventListener("abort", () => controller.close(), { once: true });
            },
          }), { headers: { "Content-Type": "text/event-stream" } });
        }
        const generationPath = url.pathname.match(/^\/api\/generation-tasks\/([0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12})(?:\/(snapshot|events))?$/i);
        if (generationPath && fixtures.generationTasks) {
          const task = fixtures.generationTasks[generationPath[1]];
          if (!task) throw new Error(`Missing generation task fixture: ${generationPath[1]}`);
          if (generationPath[2] === "events") {
            const headers = new Headers(options?.headers);
            const after = Number((headers.get("Last-Event-ID") ?? "0:0").split(":").at(-1));
            const stream = task.events.filter((event) => event.sequence > after)
              .map((event) => `id: ${event.attempt}:${event.sequence}\nevent: generation_task_event\ndata: ${JSON.stringify(event)}\n\n`).join("");
            return new Response(stream, { headers: { "Content-Type": "text/event-stream" } });
          }
          return Response.json(generationPath[2] === "snapshot" ? task.snapshot : task.summary);
        }
        const agentPath = url.pathname.match(/^\/api\/agent-threads\/([^/]+)(?:\/(messages|events))?$/);
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
        if (url.pathname.startsWith("/api/agent-approvals/") && options?.method === "POST") {
          const approvalId = url.pathname.split("/").at(-1);
          const approval = fixtures.agentSnapshots.flatMap((item) => item.approvals).find((item) => item.id === approvalId);
          if (!approval) throw new Error(`Missing Agent approval fixture: ${approvalId}`);
          Object.assign(approval, {
            status: body.decision === "approve" ? "approved" : "rejected",
            decision: body.decision,
            decided_at: new Date().toISOString(),
            decided_by_user_id: 1,
          });
          return Response.json(approval);
        }
        if (command === "get_project") {
          const projectId = JSON.parse(options.body).projectId;
          const record = fixtures.projectRecords.find((item) => item.id === projectId);
          if (!record) throw new Error(`Missing project fixture: ${projectId}`);
          return new Response(JSON.stringify(record), { headers: { "Content-Type": "application/json" } });
        }
        if (command === "update_theme") {
          const { themeId, input } = JSON.parse(options.body);
          const record = fixtures.list_themes.find((item) => item.id === themeId);
          Object.assign(record, input);
          return new Response(JSON.stringify(record), { headers: { "Content-Type": "application/json" } });
        }
        if (command === "detect_llm_config_models" && !(command in fixtures)) {
          return new Promise((resolve) => {
            window.__uiAudit.modelRequests.push({ resolve });
          });
        }
        const projectId = body.input?.project_id ?? body.input?.derived_project_id ?? body.projectId ?? body.project_id;
        const chapterId = body.input?.chapter_id ?? body.input?.source_chapter_id ?? body.chapterId ?? body.chapter_id ?? body.input?.chapter_number ?? body.chapterNumber ?? body.chapter_number;
        const profileName = body.input?.name ?? body.name;
        const profileId = body.input?.profile_id ?? body.profileId ?? body.characterId
          ?? fixtures.list_profiles?.find((profile) => profile.name === profileName)?.id;
        if (command === "get_profile_state_at_chapter" && fixtures.byProfileChapter) {
          const value = fixtures.byProfileChapter[profileId]?.[body.input?.chapter_number];
          if (!value) throw new Error(`Missing profile chapter fixture: ${profileId}/${body.input?.chapter_number}`);
          return Response.json(value);
        }
        if (command === "get_snapshot" && fixtures.bySnapshot) {
          const value = fixtures.bySnapshot[body.input?.content_hash];
          if (!value) throw new Error(`Missing snapshot fixture: ${body.input?.content_hash}`);
          return Response.json(value);
        }
        if (command === "get_project_asset_dependency_graph_node_detail" && fixtures.byAssetNode) {
          const value = fixtures.byAssetNode[body.nodeId];
          if (!value) throw new Error(`Missing asset node fixture: ${body.nodeId}`);
          return Response.json(value);
        }
        if (command === "get_primary_portrait" && fixtures.byProfile) {
          return Response.json(fixtures.list_portraits_by_project.find((portrait) => portrait.profile_id === profileId && portrait.is_primary) ?? null);
        }
        if (["load_video_playback_data", "load_video_scrub_playback_data"].includes(command) && fixtures.playbackData) {
          const value = fixtures.playbackData[fixtures.videoDurations[body.videoId]];
          if (!value) throw new Error(`Missing playback fixture: ${body.videoId}`);
          return Response.json(value);
        }
        if (["load_cached_assembled_chapter_video_preview", "assemble_chapter_videos"].includes(command) && fixtures.assemblyPreviews) {
          const metadata = fixtures.assemblyPreviews[chapterId];
          if (!metadata) throw new Error(`Missing assembly fixture: ${chapterId}`);
          return Response.json({ ...metadata, ...fixtures.playbackData[metadata.duration_secs], cached: command.startsWith("load_cached") });
        }
        const sources = [fixtures.byChapter?.[chapterId], fixtures.byProfile?.[profileId], fixtures.byProject?.[projectId], fixtures];
        const source = sources.find((item) => item && Object.hasOwn(item, command));
        if (!source) {
          window.__uiAudit.unexpected.push(command);
          return new Response(JSON.stringify({ message: `Missing UI fixture: ${command}` }), { status: 501 });
        }
        let value = source[command];
        if (Array.isArray(value)) {
          if (command === "list_images_by_projects") value = value.filter((image) => body.input.project_ids.includes(image.project_id));
          if (command === "list_images_by_project") value = value.filter((image) => image.project_id === projectId);
          if (command === "list_images_by_chapter") value = value.filter((image) => image.project_id === projectId && image.chapter_id === chapterId);
          if (command === "list_manga_pipeline_statuses") value = value.filter((status) => body.input.chapter_ids.includes(status.chapter_id));
          if (command === "list_profile_materials") {
            const { kind, filter, query } = body.input;
            value = value.filter((item) => (!kind || kind === "all" || item.kind === kind)
              && (!filter || filter === "all" || item.material_state === filter)
              && (!query || `${item.profile.name} ${item.profile.primary_text}`.includes(query)));
          }
        }
        return Response.json(value);
      };
      const clearRect = CanvasRenderingContext2D.prototype.clearRect;
      CanvasRenderingContext2D.prototype.clearRect = function (...args) {
        if (this.canvas.width === 520 && this.canvas.height === 240) {
          window.__uiAudit.draws[this.canvas.isConnected ? "attached" : "detached"]++;
        }
        return clearRect.apply(this, args);
      };
    }, encodeURIComponent(JSON.stringify({ ...uiFixtures, ...overrides })));
}
