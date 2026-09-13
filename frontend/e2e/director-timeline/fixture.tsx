import { createRoot } from "react-dom/client";
import { useState } from "react";
import { I18nProvider } from "../../src/i18n";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { ChapterWorkspace, type BottomTab } from "../../src/pages/writing-desk/ChapterWorkspace";
import { writeCurrentAccessToken, writeCurrentUserId } from "../../src/api/authStorage";
import type { ChapterVisualPlanV3 } from "../../src/api/chapterVisualPlan";
import type { ChapterSummary } from "../../src/api/chapter";
import "../../src/index.css";

// UI transport fixture only. Domain edits and persistence are tested against Rust/CLI separately.
writeCurrentUserId(1);
writeCurrentAccessToken("director-browser-fixture");
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
const view = () => ({ record: { id: 10, plan_version: 1, revision, status: "draft", authoring_mode: "collaborative", source_content_sha256: "a".repeat(64), plan_sha256: "b".repeat(64), confirmed_at: null }, plan });
const secondPlan = structuredClone(plan);
let holdEdit = false;
let releaseEdit: (() => void) | null = null;
let completedEdits = 0;
let imageGenerationCount = 0;
Object.assign(window, { holdDirectorEdit: () => { holdEdit = true; }, releaseDirectorEdit: () => releaseEdit?.() });
const originalFetch = window.fetch.bind(window);
window.fetch = async (resource, options) => {
  const url = new URL(typeof resource === "string" ? resource : resource instanceof URL ? resource.href : resource.url, location.href);
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
    case "get_chapter_visual_plan": result = { source: { chapter_number: input.source_chapter_id, title: chapter.title, content: chapter.content, content_sha256: "a".repeat(64) }, starter_plan: plan, draft: input.source_chapter_id === 2 ? { record: { ...view().record, id: 20, revision: 1 }, plan: secondPlan } : view(), confirmed: null }; break;
    case "list_images_by_projects": result = plan.points.slice(0, -1).map((point, index) => ({ id: index + 1, project_id: 1, image_type: "director_storyboard_frame", file_path: `${location.origin}/e2e/director-timeline/${index < 4 ? "street" : "interior"}.svg?frame=${index}`, prompt: point.description, model: "browser-fixture", width: 1920, height: 1080, file_size: 1000, ai_score: null, is_favorite: false, chapter_id: 1, panel_id: null, metadata_json: null, created_at: null })); break;
    case "list_scene_profiles": result = [{ id: 11, project_id: 1, name: "雨夜街道" }, { id: 22, project_id: 1, name: "旧书店内" }]; break;
    case "list_image_configs": result = [{ id: 7, name: "测试图片模型", model: "fixture-image-model", backend: "openai", base_url: "http://fixture.invalid", has_api_key: true, is_default: true, max_concurrent: 1, supported_sizes: [{ width: 1920, height: 1080 }] }]; break;
    case "preview_visual_plan_point_generation": result = { aspect_ratio: "16:9", plan_sha256: "b".repeat(64), prompt: input.options.user_description, reference_image_ids: [] }; break;
    case "save_chapter_visual_plan_draft": plan = input.plan; revision += 1; result = view(); break;
    case "edit_chapter_visual_plan_draft": {
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
      result = { resource: { kind: "chapter_visual_plan", id: "10", revision: String(revision), hash: null, label: null }, changed: true, changes: [], validation: { valid: true, diagnostics: [], mode: "draft" }, visual_plan: view() };
      Object.assign(window, { directorEditFinished: ++completedEdits });
      break;
    }
    default: return new Response(JSON.stringify({ message: `Unimplemented fixture command: ${command}` }), { status: 501 });
  }
  return new Response(JSON.stringify(result), { status: 200, headers: { "content-type": "application/json" } });
};

function Fixture() {
  const [chapterId, setChapterId] = useState(1);
  const [bottomTab, setBottomTab] = useState<BottomTab>("visual_plan");
  const [canEdit, setCanEdit] = useState(true);
  Object.assign(window, { switchDirectorChapter: setChapterId });
  Object.assign(window, { setDirectorCanEdit: setCanEdit });
  const unavailable = () => { throw new Error("This fixture only exercises the director workspace"); };
  return <I18nProvider initialLocale="zh-CN"><div className="a4n-app-shell flex h-dvh flex-col bg-book-bg">
    <main className="grid min-h-0 flex-1 p-2 sm:p-3">
      <ChapterWorkspace
        projectId={1} activeNumber={chapterId}
        activeChapter={{ ...chapter, id: chapterId, chapter_number: chapterId, title: chapterId === 1 ? chapter.title : "第二章" }}
        activeOutline={null} activeChapterContext={null} activeChapterBusy={false}
        activeChapterGeneratingMessage="" analysisBlockedReason={null}
        editorContent={chapter.content} dirty={false} savingContent={false}
        optimizationPatchPreview={null} autoScrollEditorContent={false} wordCount={chapter.word_count}
        generatingAnalysis={false} bottomTab={bottomTab} onBottomTabChange={setBottomTab}
        onEditorContentChange={unavailable} onGenerateAnalysis={unavailable}
        onPreviewGenerateAnalysis={unavailable} onResumeAnalysisSync={unavailable}
        onPreviewResumeAnalysisSync={unavailable} onResumeDerivedSync={unavailable}
        onOpenEmbeddingSettings={unavailable} canEdit={canEdit}
      />
    </main>
  </div><ConfirmDialogHost /><ToastContainer /></I18nProvider>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
