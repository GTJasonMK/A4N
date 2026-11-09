import { createRoot } from "react-dom/client";
import { useState } from "react";
import { Clapperboard, Film, FileText, ImagePlus, PanelTop, Scissors } from "lucide-react";
import { I18nProvider } from "../../src/i18n";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { ImageLightboxProvider } from "../../src/components/ui/ImageLightbox";
import { MediaStageHeader } from "../../src/components/shell/MediaStageHeader";
import { ChapterVisualPlanTab } from "../../src/pages/novel-detail/ChapterVisualPlanTab";
import { planChanges } from "../../src/pages/novel-detail/visual-plan/changes";
import { canonicalJson } from "../../src/utils/contentHash";
import { writeFixtureAuth } from "../fixtureAuth";
import type { ChapterVisualPlanV3 } from "../../src/api/chapterVisualPlan";
import type { ChapterSummary } from "../../src/api/chapter";
import "../../src/index.css";

// UI transport fixture only. Domain edits and persistence are tested against Rust/CLI separately.
await writeFixtureAuth("director-browser-fixture");
const commands: Array<{ command: string; input: unknown }> = [];
Object.assign(window, { directorCommands: commands });
let revision = 1;
let plan: ChapterVisualPlanV3 = {
  schema_version: "chapter_visual_plan_v3", frame: { width: 1920, height: 1080 },
  points: [0, 2000, 4000, 6000, 8000, 10000, 12000].map((at_ms, index) => ({
    id: `p${index}`, at_ms, label: ["雨夜街口", "走近书店", "门前停步", "推门", "进入店内", "发现来信", "结束"][index],
    image_id: index === 6 ? null : index + 1, description: "旅人在雨夜走近旧书店。", reference_image_ids: [],
  })),
  segments: [
    { id: "s1", title: "走近书店", start_point_id: "p0", end_point_id: "p3", prompt: "镜头跟随旅人走近雨夜中的旧书店，在门前停下。", scene_binding: { kind: "scene", scene_profile_id: 11 }, use_end_point_image: false },
    { id: "s2", title: "推门转场", start_point_id: "p3", end_point_id: "p4", prompt: "旅人推开门，视线从雨夜街道进入温暖店内。", scene_binding: { kind: "transition", from_scene_profile_id: 11, to_scene_profile_id: 22 }, use_end_point_image: false },
    { id: "s3", title: "发现来信", start_point_id: "p4", end_point_id: "p6", prompt: "镜头缓缓推近柜台上的来信。", scene_binding: { kind: "scene", scene_profile_id: 22 }, use_end_point_image: false },
  ],
};
const chapter: ChapterSummary = {
  id: 1, project_id: 1, chapter_number: 1, title: "雨夜来信", content: "旅人走近旧书店，推门进入，在柜台上发现一封来信。",
  word_count: 26, status: "draft", outline_id: null, analysis_data: null, style_tags: null, summary: null, created_at: null, updated_at: null,
  analysis_sync_status: null, analysis_sync_error: null, analysis_sync_can_resume: false, analysis_sync_has_saved_analysis: false, analysis_sync_resume_block_reason: null,
  derived_sync_status: "completed", derived_sync_error: null, derived_sync_can_resume: false, derived_sync_workflow_run_id: null, material_updates: [],
  material_sync: { characters: "completed", objects: "completed", scenes: "completed", content_index: "completed", outline_and_foreshadowing: "completed" },
};
const agentThread = {
  protocol_version: 1, id: "director-agent-thread", owner_user_id: 1, kind: "subagent", capability: "director",
  title: "导演编辑 Agent", status: "active", default_model_config_id: null, revision: 1,
  next_event_sequence: 1, next_turn_sequence: 2, created_at: "2026-09-15T00:00:00Z", updated_at: "2026-09-15T00:00:00Z",
  bound_resource: { kind: "chapter", id: "1", label: chapter.title, revision: null, hash: null },
  scope: { resource_kind: "chapter", resource_id: "1", project_id: 1, chapter_id: 1, allowed_capabilities: ["director"], allowed_tools: [] },
  context_summary: null,
};
const agentSnapshot = {
  protocol_version: 1,
  thread: agentThread,
  messages: [{
    id: "director-agent-message", thread_id: agentThread.id, role: "assistant", status: "complete",
    content: `# 需要注意：\n\n${Array.from({ length: 7 }, (_, index) => `${index + 1}. 第 ${index + 1} 项需要在编辑器中确认后再继续，避免覆盖当前草稿。`).join("\n")}`,
    source_kind: "agent", source_id: null, metadata: null, run_id: "director-agent-run", turn_id: "director-agent-turn",
    created_at: "2026-09-15T00:00:00Z", updated_at: "2026-09-15T00:00:00Z",
  }],
  turns: [],
  runs: [{
    id: "director-agent-run", thread_id: agentThread.id, turn_id: "director-agent-turn", attempt: 1,
    status: "running", waiting_reason: null, lease_owner: null, lease_expires_at: null,
    budget: { max_children: null, max_cost_micros: null, max_delegation_depth: null, max_duration_seconds: null, max_model_calls: null, max_parallel_tools: null, max_steps: null, max_tokens: null, max_tool_calls: null },
    steps_used: 1, model_calls_used: 1, tool_calls_used: 7, tokens_used: 900,
    context_input_tokens: 420, context_capacity_tokens: 3000, cost_micros: null, error: null,
    created_at: "2026-09-15T00:00:00Z", started_at: "2026-09-15T15:00:00Z", updated_at: "2026-09-15T15:00:00Z",
  }],
  executions: Array.from({ length: 7 }, (_, index) => ({
    id: `director-agent-execution-${index}`, run_id: "director-agent-run", sequence: index + 1,
    kind: "tool_call", tool_name: index === 0 ? "inspect_editor_target" : "read_editor_draft",
    status: "succeeded", effect: "read", replay_policy: "safe", request_digest: null,
    resource_refs: [], result_refs: [], result_summary: null, error: null,
    created_at: "2026-09-15T00:00:00Z", updated_at: "2026-09-15T00:00:00Z",
  })),
  delegations: [], approvals: [], inbox: [], memories: [], events: [],
};
const view = () => ({ edit_receipt: null, record: { id: 10, plan_version: 1, revision, status: "draft", authoring_mode: "collaborative", source_content_sha256: "a".repeat(64), plan_sha256: "b".repeat(64), confirmed_at: null }, plan });
const savedView = (before: ChapterVisualPlanV3, operationId: string) => ({ ...view(), edit_receipt: {
  sequence: revision, operation_id: operationId, project_id: 1, source_chapter_id: 1,
  source_revision_key: "fixture", author_user_id: 1,
  before: { record_id: 10, revision: revision - 1, plan_sha256: "b".repeat(64) },
  after: { record_id: 10, revision, plan_sha256: "b".repeat(64) },
  changes: planChanges(before, plan).map((change) => ({ collection: change.collection, object_id: change.id, field: change.field, before: change.before ?? null, after: change.after ?? null })),
} });
const secondPlan = structuredClone(plan);
let holdEdit = false;
let releaseEdit: (() => void) | null = null;
let completedEdits = 0;
let imageGenerationCount = 0;
const editStreams = new Set<ReadableStreamDefaultController<Uint8Array>>();
let holdSave = false;
let releaseSave: (() => void) | null = null;
const stages = [
  { id: "materials", label: "基础素材", icon: <ImagePlus size={13} /> },
  { id: "director", label: "导演编排", icon: <Clapperboard size={13} /> },
  { id: "screenplay", label: "拍摄剧本", icon: <FileText size={13} /> },
  { id: "frames", label: "片段规划", icon: <PanelTop size={13} /> },
  { id: "video", label: "片段生成", icon: <Film size={13} /> },
  { id: "assembly", label: "成片合成", icon: <Clapperboard size={13} /> },
  { id: "postprocess", label: "后期处理", icon: <Scissors size={13} /> },
];
Object.assign(window, {
  holdDirectorSave: () => { holdSave = true; },
  releaseDirectorSave: () => releaseSave?.(),
  emitDirectorPointChange: (id: string, fields: Record<string, unknown>) => {
    const before = structuredClone(plan);
    plan = { ...plan, points: plan.points.map((point) => point.id === id ? { ...point, ...fields } : point) };
    revision += 1;
    const saved = savedView(before, `remote-${revision}`);
    const bytes = new TextEncoder().encode(`id: ${revision}\nevent: visual_plan_edit\ndata: ${JSON.stringify(saved)}\n\n`);
    editStreams.forEach((controller) => controller.enqueue(bytes));
  },
});
Object.assign(window, { holdDirectorEdit: () => { holdEdit = true; }, releaseDirectorEdit: () => releaseEdit?.() });
const originalFetch = window.fetch.bind(window);
window.fetch = async (resource, options) => {
  const url = new URL(typeof resource === "string" ? resource : resource instanceof URL ? resource.href : resource.url, location.href);
  if (url.pathname === "/api/agent-runs/director-agent-run/control") {
    const input = JSON.parse(String(options?.body));
    commands.push({ command: "control_agent_run", input });
    const statuses: Record<string, string> = { pause: "paused", resume: "running", cancel: "cancelled" };
    const status = statuses[input.action];
    if (!status) return new Response("Unknown Agent control action", { status: 400 });
    agentSnapshot.runs[0].status = status;
    return new Response(JSON.stringify(agentSnapshot.runs[0]), { status: 200, headers: { "content-type": "application/json" } });
  }
  if (url.pathname === "/api/visual-plan-edits") {
    return new Response(new ReadableStream({ start(controller) {
      editStreams.add(controller);
      options?.signal?.addEventListener("abort", () => { editStreams.delete(controller); controller.close(); }, { once: true });
    } }), { headers: { "content-type": "text/event-stream" } });
  }
  if (url.pathname === "/api/agent-threads" && options?.method === "POST") {
    return new Response(JSON.stringify(agentThread), { status: 201, headers: { "content-type": "application/json" } });
  }
  if (url.pathname === "/api/agent-threads" && options?.method !== "POST") {
    return new Response(JSON.stringify([agentThread]), { status: 200, headers: { "content-type": "application/json" } });
  }
  const agentPath = url.pathname.match(/^\/api\/agent-threads\/(director-agent-thread)(?:\/(messages|events))?$/);
  if (agentPath) {
    if (agentPath[2] === "events") {
      return new Response(new ReadableStream({ start(controller) { options?.signal?.addEventListener("abort", () => controller.close(), { once: true }); } }), { headers: { "content-type": "text/event-stream" } });
    }
    if (agentPath[2] === "messages" && options?.method === "POST") {
      return new Response(JSON.stringify({ created_run: false, message_id: "director-agent-message", run_id: "director-agent-run", turn_id: "director-agent-turn", thread_id: agentThread.id, status: "completed" }), { status: 200, headers: { "content-type": "application/json" } });
    }
    return new Response(JSON.stringify(agentSnapshot), { status: 200, headers: { "content-type": "application/json" } });
  }
  if (url.pathname.endsWith("/generation-tasks/image")) {
    const input = JSON.parse(String(options?.body ?? "{}"));
    commands.push({ command: "start_image_generation", input });
    imageGenerationCount += 1;
    return new Response(JSON.stringify({ task_id: `director-image-task-${imageGenerationCount}`, attempt: 1, event_schema_version: 1, status: "reserved" }), { status: 200, headers: { "content-type": "application/json" } });
  }
  if (!url.pathname.includes("/commands/")) return originalFetch(resource, options);
  const command = decodeURIComponent(url.pathname.split("/").at(-1)!);
  const { input } = JSON.parse(String(options?.body ?? "{}"));
  commands.push({ command, input });
  if (command === "get_chapter_visual_plan" && input.source_chapter_id === 3) {
    return new Response(JSON.stringify({ code: "internal", message: "章节加载失败", retryable: false, trace_id: null }), { status: 500 });
  }
  let result: unknown;
  switch (command) {
    case "get_chapter_visual_plan": result = { scene_choices: [{ id: 11, project_id: 1, name: "雨夜街道" }, { id: 22, project_id: 1, name: "旧书店内" }], source: { chapter_number: input.source_chapter_id, title: chapter.title, content: chapter.content, content_sha256: "a".repeat(64) }, starter_plan: plan, draft: input.source_chapter_id === 2 ? { record: { ...view().record, id: 20, revision: 1 }, plan: secondPlan, edit_receipt: null } : view(), confirmed: null }; break;
    case "list_images_by_projects": result = [
      ...plan.points.slice(0, -1).map((point, index) => ({ id: index + 1, project_id: 1, image_type: "director_storyboard_frame", file_path: `${location.origin}/e2e/director-timeline/${index < 4 ? "street" : "interior"}.svg?frame=${index}`, prompt: point.description, model: "browser-fixture", width: 1920, height: 1080, file_size: 1000, ai_score: null, is_favorite: false, chapter_id: 1, panel_id: null, metadata_json: null, created_at: null })),
      ...[8, 9].map((id, index) => ({ id, project_id: 1, image_type: "director_storyboard_frame", file_path: `${location.origin}/e2e/director-timeline/street.svg?candidate=${id}`, prompt: `图片点候选 ${index + 1}`, model: "browser-fixture", width: 1920, height: 1080, file_size: 1000, ai_score: null, is_favorite: false, chapter_id: 1, panel_id: null, metadata_json: { director_candidate: { point_id: "p1" } }, created_at: `2026-09-13T00:0${index}:00Z` })),
    ]; break;
    case "list_scene_profiles": result = [{ id: 11, project_id: 1, name: "雨夜街道" }, { id: 22, project_id: 1, name: "旧书店内" }]; break;
    case "list_image_configs": result = [{ id: 7, name: "测试图片模型", model: "fixture-image-model", backend: "openai", base_url: "http://fixture.invalid", has_api_key: true, is_default: true, max_concurrent: 1, supported_sizes: [{ width: 1920, height: 1080 }] }]; break;
    case "preview_visual_plan_point_generation": result = { aspect_ratio: "16:9", plan_sha256: "b".repeat(64), prompt: input.options.user_description, reference_image_ids: [] }; break;
    case "save_chapter_visual_plan_draft": {
      const before = plan; plan = input.plan; revision += 1; result = savedView(before, input.operation_id);
      if (holdSave) {
        holdSave = false;
        Object.assign(window, { directorSaveHeld: true });
        await new Promise<void>((resolve) => { releaseSave = resolve; });
        Object.assign(window, { directorSaveHeld: false });
      }
      break;
    }
    case "edit_chapter_visual_plan_draft": {
      const before = structuredClone(plan);
      if (holdEdit) { holdEdit = false; await new Promise<void>((resolve) => { releaseEdit = resolve; }); }
      if (input.operation.kind === "scale_timeline") {
        const ratio = input.operation.duration_ms / plan.points.at(-1)!.at_ms;
        plan = { ...plan, points: plan.points.map((point) => ({ ...point, at_ms: Math.round(point.at_ms * ratio) })) };
      } else if (input.operation.kind === "move_point") {
        plan = { ...plan, points: plan.points.map((point) => point.id === input.operation.point_id ? { ...point, at_ms: input.operation.at_ms } : point) };
      } else if (input.operation.kind === "move_segment_boundary") {
        const index = plan.segments.findIndex((segment) => segment.id === input.operation.left_segment_id);
        plan = { ...plan, segments: plan.segments.map((segment, offset) => offset === index ? { ...segment, end_point_id: input.operation.point_id } : offset === index + 1 ? { ...segment, start_point_id: input.operation.point_id } : segment) };
      } else if (input.operation.kind === "set_frame") {
        plan = { ...plan, frame: input.operation.frame };
      } else return new Response(JSON.stringify({ message: "UI fixture does not implement this domain edit" }), { status: 501 });
      revision += 1;
      result = { resource: { kind: "chapter_visual_plan", id: "10", revision: String(revision), hash: null, label: null }, changed: true, changes: [], validation: { valid: true, diagnostics: [], mode: "draft" }, visual_plan: savedView(before, input.operation_id) };
      Object.assign(window, { directorEditFinished: ++completedEdits });
      break;
    }
    default: return new Response(JSON.stringify({ message: `Unimplemented fixture command: ${command}` }), { status: 501 });
  }
  // A worker response may reorder object keys without changing the saved plan.
  const body = command === "save_chapter_visual_plan_draft" ? canonicalJson(result) : JSON.stringify(result);
  return new Response(body, { status: 200, headers: { "content-type": "application/json" } });
};

function Fixture() {
  const [chapterId, setChapterId] = useState(1);
  const [canEdit, setCanEdit] = useState(true);
  const [workspaceStage, setWorkspaceStage] = useState("director");
  Object.assign(window, { switchDirectorChapter: setChapterId });
  Object.assign(window, { setDirectorCanEdit: setCanEdit });
  return <I18nProvider initialLocale="zh-CN"><ImageLightboxProvider><div className="a4n-app-shell flex h-dvh flex-col bg-book-bg">
    <header className="shrink-0 px-3 pt-2">
      <MediaStageHeader id="video-stages-3" label="视频工作区" tabs={stages} activeTab={workspaceStage}
        onTabChange={(stage) => { setWorkspaceStage(stage); return true; }} />
    </header>
    <main className="grid min-h-0 flex-1 p-2 sm:p-3">
      <ChapterVisualPlanTab
        projectId={1} canEdit={canEdit}
        chapter={{ ...chapter, id: chapterId, chapter_number: chapterId, title: chapterId === 1 ? chapter.title : "第二章" }}
      />
    </main>
  </div><ConfirmDialogHost /><ToastContainer /></ImageLightboxProvider></I18nProvider>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
