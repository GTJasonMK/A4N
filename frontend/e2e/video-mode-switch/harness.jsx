import React from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "../../src/i18n";
import {
  IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
  IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
} from "../../src/api/image";
import { ConfirmDialogHost } from "../../src/components/feedback/ConfirmDialog";
import { PromptDialogHost } from "../../src/components/feedback/PromptDialog";
import { ToastContainer } from "../../src/components/feedback/Toast";
import { ImageLightboxProvider } from "../../src/components/ui/ImageLightbox";
import { ProgressTaskProvider } from "../../src/hooks/useProgressTask";
import { VideoTab } from "../../src/pages/novel-detail/VideoTab";
import { videoSegmentRecipeFrameFingerprintJson, videoSegmentRecipeGenerationFingerprintJson } from "../../src/pages/novel-detail/video/videoSegmentPlan";
import { sha256Hex } from "../../src/utils/contentHash";
import { initializePortableStorage } from "../../src/utils/portableStorage";
import "../../src/index.css";

const projectId = 1;
const chapterId = 2;
const planId = 41;
const scenario =
  new URLSearchParams(window.location.search).get("scenario") ?? "mode-switch";
const cancelResumeScenario = scenario === "cancel-resume";
const planningContentRunningScenario =
  scenario === "planning-content-running";
const segmentPlanGenerationScenario =
  cancelResumeScenario || planningContentRunningScenario;
const parallelRecipeSavesScenario = scenario === "parallel-recipe-saves";
const crossModeContinuityScenario = scenario === "cross-mode-continuity";
const stalePlanAdoptionScenario = scenario === "stale-plan-adoption";
const mixedAssemblyGapScenario = scenario === "mixed-assembly-gap";
const mixedAssemblyReadyScenario = scenario === "mixed-assembly-ready";
const mixedAssemblyScenario =
  mixedAssemblyGapScenario || mixedAssemblyReadyScenario;
const referenceMediaLayoutScenario = scenario === "reference-media-layout";
const referenceMediaSaveFailureScenario =
  scenario === "reference-media-save-failure";
const referenceMediaAdoptedVideoScenario =
  scenario === "reference-media-adopted-video";
const referenceMediaScenario =
  referenceMediaLayoutScenario || referenceMediaSaveFailureScenario;
const segmentSelectionScenario = scenario === "segment-selection";
const storyboardAssetsScenario = scenario === "storyboard-assets";
const content = "真实浏览器模式切换测试正文";
const shootingScript = {
  schema_version: "chapter_shooting_script_v2",
  scenes: [
    {
      scene_index: 1,
      narrative_unit_index: 1,
      title: "测试场景",
      shots: [{ shot_id: "unit_1_shot_1" }, { shot_id: "unit_2_shot_1" }],
    },
  ],
};
const contentHash = [
  ...new Uint8Array(
    await crypto.subtle.digest("SHA-256", new TextEncoder().encode(content)),
  ),
]
  .map((byte) => byte.toString(16).padStart(2, "0"))
  .join("");

const recipe = (segmentNumber, workflowMode, overrides = {}) => ({
  schema_version: "video_segment_execution_recipe_v1",
  segment_number: segmentNumber,
  workflow_mode: workflowMode,
  style: "cinematic",
  prompt_output_language: "zh_cn",
  duration_secs: 6,
  duration_secs_options: [6],
  aspect_ratio: "16:9",
  reference_image_ids: [],
  video_library_image_ids: [],
  guidance_mode:
    workflowMode === "chat_video" || workflowMode === "reference_media"
      ? "none"
      : "system_frames",
  guidance_reference_image_ids: [],
  continuity_source_video_id: null,
  reference_media: null,
  adopted_video_id: null,
  ...overrides,
});

const sourceVideoId = 501;
const currentVideoId = 502;
const staleVideoId = 503;
const selectedSegment1VideoId = 701;
const selectedSegment1CandidateVideoId = 702;
const selectedSegment2VideoId = 704;
const selectedSegment2CandidateVideoId = 705;
const referenceImageId = 701;
const referenceMediaVideoId = 709;
const assemblySegment1VideoId = 601;
const assemblySegment2VideoId = 602;
const assemblyStaleSegment2VideoId = 603;
const stalePlanId = 29;
const finalPromptId = 61;
const usesChatSegment1 =
  stalePlanAdoptionScenario ||
  mixedAssemblyScenario ||
  segmentSelectionScenario;
const firstRecipe = recipe(
  1,
  referenceMediaScenario
    ? "reference_media"
    : usesChatSegment1
      ? "chat_video"
      : "boundary_frames",
  {
    adopted_video_id: crossModeContinuityScenario
      ? sourceVideoId
      : mixedAssemblyScenario
        ? assemblySegment1VideoId
        : segmentSelectionScenario
          ? selectedSegment1VideoId
          : null,
  },
);
const secondRecipe = recipe(
  2,
  referenceMediaAdoptedVideoScenario
    ? "reference_media"
    : mixedAssemblyScenario || segmentSelectionScenario
      ? "chat_video"
      : "storyboard",
  {
    adopted_video_id: mixedAssemblyReadyScenario
      ? assemblySegment2VideoId
      : mixedAssemblyGapScenario
        ? assemblyStaleSegment2VideoId
        : segmentSelectionScenario
          ? selectedSegment2VideoId
          : referenceMediaAdoptedVideoScenario
            ? referenceMediaVideoId
            : null,
  },
);

const referenceImage = {
  id: referenceImageId,
  project_id: projectId,
  image_type: "generated",
  file_path:
    "data:image/gif;base64,R0lGODlhAQABAIAAAAAAAP///ywAAAAAAQABAAACAUwAOw==",
  prompt: "浏览器测试参考图",
  model: "browser-test",
  width: 1280,
  height: 720,
  file_size: null,
  ai_score: null,
  is_favorite: false,
  chapter_id: null,
  panel_id: null,
  metadata_json: null,
  created_at: null,
};

const planIdentity = {
  contract_epoch: 5,
  shooting_script_identity: "script-1",
  projection_sha256: "projection-1",
  segment_plan_sha256: "plan-1",
  unit_plan_sha256: "unit-1",
  source_content_sha256: contentHash,
  style_sha256: "style-1",
  duration_secs: 6,
  aspect_ratio: "16:9",
  segment_identities: [
    {
      source_shot_id: "unit_1_shot_1",
      segment_index_in_shot: 1,
      segment_sha256: "segment-1",
    },
    {
      source_shot_id: "unit_2_shot_1",
      segment_index_in_shot: 1,
      segment_sha256: "segment-2",
    },
  ],
};

let plan = cancelResumeScenario
  ? null
  : {
      id: planId,
      project_id: projectId,
      chapter_id: chapterId,
      style: "cinematic",
      prompt_type: "frame_segment_plan",
      workflow_mode: "shared_segment_plan",
      shot_number: 0,
      title: null,
      scene_summary: null,
      visual_prompt: "plan",
      motion_prompt: null,
      negative_prompt: null,
      duration_secs: 6,
      aspect_ratio: "16:9",
      reference_image_ids_json: null,
      status: "completed",
      generated_video_id: null,
      metadata_json: {
        contract_version: "video_execution_contract_v5",
        source: { content_sha256: contentHash },
        video_execution_identity: planIdentity,
        segments: [
          {
            segment_number: 1,
            title: "片段一",
            scene_summary: "片段一的场景摘要",
            duration_secs: 6,
            aspect_ratio: "16:9",
            source_shot_id: "unit_1_shot_1",
            segment_index_in_shot: 1,
          },
          {
            segment_number: 2,
            title: "片段二",
            scene_summary: "片段二的场景摘要",
            duration_secs: 6,
            aspect_ratio: "16:9",
            source_shot_id: "unit_2_shot_1",
            segment_index_in_shot: 1,
          },
        ],
        segment_recipes: [firstRecipe, secondRecipe],
      },
      updated_at: "2026-07-30T00:00:00Z",
    };

if (planningContentRunningScenario && plan) {
  plan = {
    ...plan,
    metadata_json: {
      ...plan.metadata_json,
      video_execution_identity: {
        ...plan.metadata_json.video_execution_identity,
        segment_identities:
          plan.metadata_json.video_execution_identity.segment_identities.slice(
            0,
            1,
          ),
      },
      segments: plan.metadata_json.segments.slice(0, 1),
      segment_recipes: plan.metadata_json.segment_recipes.slice(0, 1),
    },
  };
}

const firstRecipeFingerprint =
  crossModeContinuityScenario ||
  stalePlanAdoptionScenario ||
  mixedAssemblyScenario ||
  segmentSelectionScenario
    ? await sha256Hex(videoSegmentRecipeGenerationFingerprintJson(firstRecipe))
    : null;
const secondRecipeFingerprint =
  mixedAssemblyScenario ||
  segmentSelectionScenario ||
  referenceMediaAdoptedVideoScenario
    ? await sha256Hex(videoSegmentRecipeGenerationFingerprintJson(secondRecipe))
    : null;
const secondFrameRecipeFingerprint = storyboardAssetsScenario
  ? await sha256Hex(videoSegmentRecipeFrameFingerprintJson(secondRecipe))
  : null;

let storyboardPrompt = storyboardAssetsScenario
  ? {
      id: 229,
      project_id: projectId,
      chapter_id: chapterId,
      style: "cinematic",
      prompt_type: "storyboard_prompt",
      workflow_mode: "storyboard",
      shot_number: 2,
      title: "片段二分镜板",
      scene_summary: "已生成的分镜板",
      visual_prompt: "storyboard",
      motion_prompt: null,
      negative_prompt: null,
      duration_secs: 6,
      aspect_ratio: "16:9",
      reference_image_ids_json: null,
      status: "completed",
      generated_video_id: null,
      metadata_json: {
        contract_version: "storyboard_execution_unit_v2",
        source_prompt_type: "frame_segment_plan",
        prompt_output_language: "zh_cn",
        source: { content_sha256: contentHash },
        video_execution_identity: {
          ...planIdentity,
          segment_plan_prompt_id: planId,
          segment_number: 2,
          source_shot_id: "unit_2_shot_1",
          segment_index_in_shot: 1,
          segment_sha256: "segment-2",
          segment_recipe_fingerprint: secondFrameRecipeFingerprint,
        },
      },
      updated_at: "2026-07-30T00:00:00Z",
    }
  : null;

let storyboardImage = storyboardAssetsScenario
  ? {
      ...referenceImage,
      id: 179,
      image_type: IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
      chapter_id: chapterId,
      prompt: "片段二分镜板",
      metadata_json: {
        generation_context: {
          source: IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
          prompt_id: 229,
          shot_number: 2,
        },
      },
    }
  : null;

const makeVideo = ({
  id,
  workflowMode,
  videoType = "chapter_video",
  prompt,
  segmentPlanPromptId,
  segmentNumber,
  recipeFingerprint,
  status = "completed",
  filePath = `/tmp/segment-${id}.mp4`,
  errorMessage = null,
}) => ({
  id,
  project_id: projectId,
  chapter_id: chapterId,
  video_type: videoType,
  workflow_mode: workflowMode,
  file_path: filePath,
  thumbnail_path: null,
  prompt,
  negative_prompt: null,
  model: "browser-test",
  width: 1280,
  height: 720,
  duration_secs: 6,
  fps: 24,
  file_size: 1024,
  source_video_id: null,
  source_image_id: null,
  status,
  provider_job_id: status === "running" ? `job-${id}` : null,
  error_message: errorMessage,
  metadata_json: {
    shot_number: segmentNumber,
    video_execution_identity: {
      ...planIdentity,
      segment_plan_prompt_id: segmentPlanPromptId,
      segment_number: segmentNumber,
      source_shot_id: `unit_${segmentNumber}_shot_1`,
      segment_index_in_shot: 1,
      segment_sha256: `segment-${segmentNumber}`,
      segment_recipe_fingerprint: recipeFingerprint,
    },
  },
  is_favorite: false,
  created_at: null,
  updated_at: null,
});

const sourceVideo = crossModeContinuityScenario
  ? makeVideo({
      id: sourceVideoId,
      workflowMode: "boundary_frames",
      prompt: "片段 1 首尾帧视频",
      segmentPlanPromptId: planId,
      segmentNumber: 1,
      recipeFingerprint: firstRecipeFingerprint,
    })
  : null;

const currentAdoptionVideo = stalePlanAdoptionScenario
  ? makeVideo({
      id: currentVideoId,
      workflowMode: "chat_video",
      prompt: "当前规划候选视频",
      segmentPlanPromptId: planId,
      segmentNumber: 1,
      recipeFingerprint: firstRecipeFingerprint,
    })
  : null;

const stalePlanVideo = stalePlanAdoptionScenario
  ? makeVideo({
      id: staleVideoId,
      workflowMode: "chat_video",
      prompt: "旧规划遗留视频",
      segmentPlanPromptId: stalePlanId,
      segmentNumber: 1,
      recipeFingerprint: firstRecipeFingerprint,
    })
  : null;

const assemblySegment1Video = mixedAssemblyScenario
  ? makeVideo({
      id: assemblySegment1VideoId,
      workflowMode: "chat_video",
      prompt: "成片片段 1 有效视频",
      segmentPlanPromptId: planId,
      segmentNumber: 1,
      recipeFingerprint: firstRecipeFingerprint,
    })
  : null;

const assemblySegment2Video = mixedAssemblyScenario
  ? makeVideo({
      id: assemblySegment2VideoId,
      workflowMode: "chat_video",
      prompt: "成片片段 2 有效视频",
      segmentPlanPromptId: planId,
      segmentNumber: 2,
      recipeFingerprint: secondRecipeFingerprint,
    })
  : null;

const assemblyStaleSegment2Video = mixedAssemblyGapScenario
  ? makeVideo({
      id: assemblyStaleSegment2VideoId,
      workflowMode: "chat_video",
      prompt: "成片片段 2 旧规划视频",
      segmentPlanPromptId: stalePlanId,
      segmentNumber: 2,
      recipeFingerprint: secondRecipeFingerprint,
    })
  : null;

const selectedSegment1CandidateVideo = segmentSelectionScenario
  ? makeVideo({
      id: selectedSegment1CandidateVideoId,
      workflowMode: "chat_video",
      prompt: "片段 1 普通候选视频",
      segmentPlanPromptId: planId,
      segmentNumber: 1,
      recipeFingerprint: firstRecipeFingerprint,
    })
  : null;

const selectedSegment1Video = segmentSelectionScenario
  ? makeVideo({
      id: selectedSegment1VideoId,
      workflowMode: "chat_video",
      prompt: "片段 1 已采用视频",
      segmentPlanPromptId: planId,
      segmentNumber: 1,
      recipeFingerprint: firstRecipeFingerprint,
    })
  : null;

const selectedSegment2CandidateVideo = segmentSelectionScenario
  ? makeVideo({
      id: selectedSegment2CandidateVideoId,
      workflowMode: "chat_video",
      prompt: "片段 2 普通候选视频",
      segmentPlanPromptId: planId,
      segmentNumber: 2,
      recipeFingerprint: secondRecipeFingerprint,
    })
  : null;

const selectedSegment2Video = segmentSelectionScenario
  ? makeVideo({
      id: selectedSegment2VideoId,
      workflowMode: "chat_video",
      prompt: "片段 2 已采用视频",
      segmentPlanPromptId: planId,
      segmentNumber: 2,
      recipeFingerprint: secondRecipeFingerprint,
    })
  : null;

const referenceMediaVideo = referenceMediaAdoptedVideoScenario
  ? makeVideo({
      id: referenceMediaVideoId,
      workflowMode: "reference_media",
      videoType: "reference_media_video",
      prompt: "片段 2 已采用参考媒体视频",
      segmentPlanPromptId: planId,
      segmentNumber: 2,
      recipeFingerprint: secondRecipeFingerprint,
    })
  : null;

let finalPrompt = stalePlanAdoptionScenario
  ? {
      id: finalPromptId,
      project_id: projectId,
      chapter_id: chapterId,
      style: "cinematic",
      prompt_type: "video_prompt",
      workflow_mode: "chat_video",
      shot_number: 1,
      title: null,
      scene_summary: null,
      visual_prompt: "最终提示词",
      motion_prompt: null,
      negative_prompt: null,
      duration_secs: 6,
      aspect_ratio: "16:9",
      reference_image_ids_json: null,
      status: "completed",
      generated_video_id: null,
      metadata_json: {
        contract_version: "chat_video_execution_v4",
        source_prompt_type: "frame_segment_plan",
        prompt_output_language: "zh_cn",
        source: { content_sha256: contentHash },
        video_execution_identity: {
          ...planIdentity,
          segment_plan_prompt_id: planId,
          segment_number: 1,
          source_shot_id: "unit_1_shot_1",
          segment_index_in_shot: 1,
          segment_sha256: "segment-1",
          segment_recipe_fingerprint: firstRecipeFingerprint,
          frame_set_sha256: "frames-1",
          prompt_output_language: "zh_cn",
          generation_asset_fingerprint: {},
        },
      },
      updated_at: "2026-07-30T00:00:00Z",
    }
  : null;

const chapterVideos = () => {
  const rows = [];
  if (sourceVideo) rows.push(structuredClone(sourceVideo));
  if (currentAdoptionVideo) rows.push(structuredClone(currentAdoptionVideo));
  if (stalePlanVideo) rows.push(structuredClone(stalePlanVideo));
  if (assemblySegment1Video) rows.push(structuredClone(assemblySegment1Video));
  if (assemblySegment2Video) rows.push(structuredClone(assemblySegment2Video));
  if (assemblyStaleSegment2Video)
    rows.push(structuredClone(assemblyStaleSegment2Video));
  if (selectedSegment1CandidateVideo)
    rows.push(structuredClone(selectedSegment1CandidateVideo));
  if (selectedSegment1Video) rows.push(structuredClone(selectedSegment1Video));
  if (selectedSegment2CandidateVideo)
    rows.push(structuredClone(selectedSegment2CandidateVideo));
  if (selectedSegment2Video) rows.push(structuredClone(selectedSegment2Video));
  if (referenceMediaVideoPublished && referenceMediaVideo)
    rows.push(structuredClone(referenceMediaVideo));
  return rows;
};

const chapterPrompts = () => {
  const rows = [];
  if (plan) rows.push(structuredClone(plan));
  if (finalPrompt) rows.push(structuredClone(finalPrompt));
  if (storyboardPrompt) rows.push(structuredClone(storyboardPrompt));
  return rows;
};

const updatePlanRecipe = (nextRecipe) => {
  if (!plan) throw new Error("plan missing");
  plan = {
    ...plan,
    metadata_json: {
      ...plan.metadata_json,
      segment_recipes: plan.metadata_json.segment_recipes.map((current) =>
        current.segment_number === nextRecipe.segment_number
          ? structuredClone(nextRecipe)
          : current,
      ),
    },
  };
};

const videoById = (videoId) =>
  chapterVideos().find((video) => video.id === videoId) ?? null;

const workflowRunId = 71;
const workflowOptions = (options) => ({
  style: options.style,
  prompt_output_language: options.prompt_output_language,
  prompt_batch_size: 1,
  segment_plan_prompt_id: null,
  target_shot_number: null,
  target_narrative_unit_index: options.target_narrative_unit_index,
  segment_recipe_fingerprint: null,
  duration_secs: options.duration_secs,
  duration_secs_options: [...options.duration_secs_options],
  aspect_ratio: options.aspect_ratio,
  reference_image_ids: [],
  video_library_image_ids: [],
  regenerate_existing: options.regenerate_existing,
});

const checkpointSegment = (shotIndex) => ({
  segment_number: 1,
  title: `片段 ${shotIndex}`,
  scene_summary: `镜头 ${shotIndex} 的已保存片段`,
  duration_secs: 6,
  aspect_ratio: "16:9",
  source_shot_id: `unit_${shotIndex}_shot_1`,
});

const checkpoint = (shotIndex) => ({
  id: 100 + shotIndex,
  workflow_run_id: workflowRunId,
  step_key: `video_segment_plan.shot.unit_${shotIndex}_shot_1`,
  step_order: shotIndex,
  status: "completed",
  input_fingerprint_json: { extra: { shot_index: shotIndex, total_shots: 2 } },
  checkpoint_json: { parsed_output: [checkpointSegment(shotIndex)] },
  artifacts_json: { segment_recipes: [recipe(1, "chat_video")] },
  generation_run_ids_json: [],
  error_message: null,
  created_at: "2026-07-30T00:00:00Z",
  updated_at: "2026-07-30T00:00:00Z",
  completed_at: "2026-07-30T00:00:00Z",
});

let run = null;
let checkpoints = [];
let generationNumber = 0;
const generationTasks = new Map();
let nextCallbackId = 1;

const segmentPlanDraftDetail = () => {
  const ordered = [...checkpoints].sort(
    (left, right) =>
      left.input_fingerprint_json.extra.shot_index -
      right.input_fingerprint_json.extra.shot_index,
  );
  const recipes = [];
  let segmentNumber = 0;
  const segments = ordered.flatMap((row) =>
    row.checkpoint_json.parsed_output.map((segment, index, shotSegments) => {
      segmentNumber += 1;
      recipes.push({
        ...row.artifacts_json.segment_recipes[index],
        segment_number: segmentNumber,
      });
      return {
        ...segment,
        segment_number: segmentNumber,
        segment_index_in_shot: index + 1,
        shot_segment_count: shotSegments.length,
      };
    }),
  );
  return {
    run: structuredClone(run),
    segments,
    recipes,
    completed_shot_count: ordered.length,
    total_shot_count: ordered[0]?.input_fingerprint_json.extra.total_shots ?? 0,
  };
};

const handleFor = (number) => ({
  event_schema_version: 1,
  task_id: `00000000-0000-4000-8000-${String(number).padStart(12, "0")}`,
  attempt: 1,
  status: "running",
});

const emit = (task, kind, data) => {
  task.sequence += 1;
  task.channel.onmessage({
    schema_version: 1,
    task_id: task.handle.task_id,
    attempt: task.handle.attempt,
    sequence: task.sequence,
    emitted_at_ms: Date.now(),
    event: { kind, data },
  });
};

const startedData = (task) => ({
  task_kind: "video_prompt_segment_plan",
  delivery_mode: "stream",
  root_task_id: task.handle.task_id,
  project_id: projectId,
  root_project_id: projectId,
  workflow_run_id: workflowRunId,
  target: {
    kind: "video_prompt_segment_plan",
    id: `${projectId}:chapter:${chapterId}:prompt_segment_plan:shared`,
    revision: null,
  },
});

const progressData = (shotIndex) => ({
  stage_code: "chapter_video_segment_plan",
  phase_kind: "save",
  message: `正在生成镜头 ${shotIndex}`,
  severity: "success",
  estimated: false,
  current: shotIndex,
  total: 2,
  item_key: `unit_${shotIndex}_shot_1`,
  item_index: shotIndex,
  item_total: 2,
  unit_label: "镜头",
});

const cancelTask = (task) => {
  task.handle = { ...task.handle, status: "cancelled" };
  run = {
    ...run,
    status: "cancelled",
    can_resume: true,
    active_task_id: null,
    current_step_key: checkpoints.at(-1)?.step_key ?? null,
    progress: {
      current: checkpoints.length,
      total: 2,
      percent: checkpoints.length * 50,
      message: `已保存镜头 ${checkpoints.length}`,
      meta: null,
      updated_at: "2026-07-30T00:00:00Z",
    },
  };
  setTimeout(() => {
    emit(task, "cancel_requested", { reason: "user_requested" });
    emit(task, "cancelled", {
      reason: "user_requested",
      draft_disposition: "retained_until_expiry",
      batch_summary: null,
      summary: `已停止，已保存镜头 ${checkpoints.length}`,
    });
  }, 0);
};

const calls = [];
let referenceMediaVideoPublished = false;
let releaseParallelFirstSave = null;
let releaseCrossModeSourceRecipeSave = null;
const callSnapshot = (value) => JSON.parse(JSON.stringify(value));
const invoke = async (command, args = {}) => {
  if (command === "invoke_host_command") {
    if (args.userId !== 1) throw new Error("unexpected fixture caller");
    return invoke(args.command, args.args);
  }
  calls.push({ command, args: callSnapshot(args) });
  switch (command) {
    case "get_all_frontend_state":
      return {
        "a4n:current_user_id": "1",
        [`a4n:video:${projectId}:${chapterId}:prompt:${planId}:selected-segment`]:
          storyboardAssetsScenario ? "2" : "1",
        ...(segmentPlanGenerationScenario
          ? { [`a4n:video:${projectId}:${chapterId}:workspace-tab`]: "frames" }
          : {}),
        ...(mixedAssemblyScenario
          ? {
              [`a4n:video:${projectId}:${chapterId}:workspace-tab`]: "assembly",
            }
          : {}),
      };
    case "set_frontend_state_many":
    case "remove_frontend_state_many":
      return null;
    case "list_video_reference_images":
      return storyboardImage
        ? [referenceImage, storyboardImage]
        : [referenceImage];
    case "delete_image":
      if (!storyboardImage || args.input?.image_id !== storyboardImage.id) {
        throw new Error("unexpected delete_image input");
      }
      storyboardImage = null;
      return null;
    case "list_video_configs":
    case "list_image_configs":
    case "list_active_generation_tasks":
      return [];
    case "list_generation_task_history":
      if (
        !segmentPlanGenerationScenario ||
        args.input?.project_id !== projectId ||
        args.input?.limit !== 100
      ) {
        throw new Error("unexpected list_generation_task_history input");
      }
      return {
        items: [...generationTasks.values()].reverse().map((task) => ({
          handle: structuredClone(task.handle),
          started: startedData(task),
          updated_at_ms: Date.now(),
          result_ref: null,
          snapshot_state: "available",
          terminal_event: null,
        })),
        next_cursor: null,
      };
    case "get_chapter_shooting_script":
      return { record: {}, script: shootingScript };
    case "get_chapter_shooting_script_unit_progress":
      return null;
    case "get_chapter_video_timeline":
      return {
        id: 1,
        revision: 1,
        items: mixedAssemblyScenario
          ? plan.metadata_json.segments.map((segment) => ({
              id: 1000 + segment.segment_number,
              kind: "native",
              display_index: segment.segment_number,
              segment_number: segment.segment_number,
              source_shot_id: segment.source_shot_id,
              plan_prompt_id: planId,
              generated_video_id: plan.metadata_json.segment_recipes.find(
                (item) => item.segment_number === segment.segment_number,
              ).adopted_video_id,
              status: "active",
              title: segment.title,
            }))
          : [],
      };
    case "list_videos_by_chapter":
      return chapterVideos();
    case "list_workflow_runs":
      return run ? [structuredClone(run)] : [];
    case "list_chapter_video_prompts": {
      const rows = chapterPrompts();
      if (releaseCrossModeSourceRecipeSave) {
        calls.push({ command: "post_select_refresh_started", args: {} });
        releaseCrossModeSourceRecipeSave();
        releaseCrossModeSourceRecipeSave = null;
      }
      return rows;
    }
    case "load_video_playback_data":
    case "load_video_scrub_playback_data":
      return {
        mime_type: "video/mp4",
        data_base64: "AAAA",
      };
    case "load_cached_assembled_chapter_video_preview":
      return null;
    case "assemble_chapter_videos": {
      if (!mixedAssemblyScenario) {
        throw new Error("unexpected assemble_chapter_videos call");
      }
      if (args.workflowMode !== "mixed") {
        throw new Error(
          `unexpected assemble workflowMode: ${args.workflowMode}`,
        );
      }
      return {
        cached: false,
        clips: [
          {
            video_id: assemblySegment1VideoId,
            duration_secs: 6,
            file_path: "/tmp/segment-601.mp4",
            insert_after_video_id: null,
            shot_number: 1,
            sort_order: 1000,
            video_type: "chapter_video",
          },
          ...(mixedAssemblyReadyScenario
            ? [
                {
                  video_id: assemblySegment2VideoId,
                  duration_secs: 6,
                  file_path: "/tmp/segment-602.mp4",
                  insert_after_video_id: null,
                  shot_number: 2,
                  sort_order: 2000,
                  video_type: "chapter_video",
                },
              ]
            : []),
        ],
        data_base64: "AAAA",
        duration_secs: mixedAssemblyReadyScenario ? 12 : 6,
        file_path: "/tmp/mixed-assembly.mp4",
        file_size: 2048,
        mime_type: "video/mp4",
        updated_at: "2026-07-31T00:00:00Z",
        video_ids: [
          assemblySegment1VideoId,
          ...(mixedAssemblyReadyScenario ? [assemblySegment2VideoId] : []),
        ],
      };
    }
    case "select_segment_video": {
      if (!stalePlanAdoptionScenario || !plan) {
        throw new Error("unexpected select_segment_video input");
      }
      const videoId = args.videoId;
      const targetSegment = args.input?.target_segment;
      const segmentPlanPromptId = args.input?.segment_plan_prompt_id;
      const video = videoById(videoId);
      const identity = video?.metadata_json?.video_execution_identity;
      if (
        segmentPlanPromptId !== planId ||
        targetSegment?.source_shot_id !== "unit_1_shot_1" ||
        targetSegment?.segment_index_in_shot !== 1 ||
        !video ||
        identity?.segment_plan_prompt_id !== planId ||
        identity?.segment_number !== 1 ||
        identity?.segment_recipe_fingerprint !== firstRecipeFingerprint
      ) {
        throw new Error("旧规划或失效身份视频不能被当前规划采用");
      }
      const currentRecipe = plan.metadata_json.segment_recipes.find(
        (row) => row.segment_number === 1,
      );
      updatePlanRecipe({
        ...currentRecipe,
        adopted_video_id: videoId,
      });
      calls.push({
        command: "persisted_adoption_snapshot",
        videoId,
        adopted: Object.fromEntries(
          plan.metadata_json.segment_recipes.map((row) => [
            row.segment_number,
            row.adopted_video_id,
          ]),
        ),
      });
      return structuredClone(video);
    }
    case "load_video_segment_plan_draft":
      if (
        !segmentPlanGenerationScenario ||
        !run ||
        args.input?.workflow_run_id !== workflowRunId
      ) {
        throw new Error("unexpected load_video_segment_plan_draft input");
      }
      return segmentPlanDraftDetail();
    case "update_video_segment_plan_draft_recipe": {
      const input = args.input;
      const checkpointIndex = checkpoints.findIndex(
        (row) =>
          row.step_key === `video_segment_plan.shot.${input?.source_shot_id}`,
      );
      if (
        !cancelResumeScenario ||
        input?.workflow_run_id !== workflowRunId ||
        input?.segment_index_in_shot !== 1 ||
        checkpointIndex < 0 ||
        !input.recipe
      ) {
        throw new Error(
          "unexpected update_video_segment_plan_draft_recipe input",
        );
      }
      const current = checkpoints[checkpointIndex];
      checkpoints[checkpointIndex] = {
        ...current,
        artifacts_json: {
          ...current.artifacts_json,
          segment_recipes: [
            { ...structuredClone(input.recipe), segment_number: 1 },
          ],
        },
      };
      calls.push({
        command: "persisted_draft_recipe_snapshot",
        mode: input.recipe.workflow_mode,
      });
      return null;
    }
    case "start_video_generation": {
      if (
        !segmentPlanGenerationScenario ||
        args.input?.operation?.kind !== "prompt_segment_plan"
      ) {
        throw new Error("unexpected start_video_generation input");
      }
      const previousTaskId =
        [...generationTasks.values()].at(-1)?.handle.task_id ?? null;
      const expectedRetryTaskId =
        generationNumber === 0 ? null : previousTaskId;
      if (args.input?.meta?.retry_of_task_id !== expectedRetryTaskId) {
        throw new Error("segment plan resume must retry the previous task");
      }
      generationNumber += 1;
      const handle = handleFor(generationNumber);
      const options = args.input.operation.options;
      if (generationNumber === 1) {
        run = {
          id: workflowRunId,
          business_key: "video-segment-plan-e2e",
          workflow_kind: "video_segment_plan",
          project_id: projectId,
          root_project_id: projectId,
          source_snapshot_id: null,
          scope_table: "chapters",
          scope_id: chapterId,
          chapter_id: chapterId,
          mode: "shared_segment_plan",
          input_fingerprint_json: {
            schema_version: 1,
            workflow_kind: "video_segment_plan",
            project_id: projectId,
            chapter_id: chapterId,
            workflow_mode: "shared_segment_plan",
            options: workflowOptions(options),
          },
          status: "running",
          current_step_key: null,
          progress: null,
          active_task_id: handle.task_id,
          can_resume: false,
          resume_block_reason: null,
          error_message: null,
          created_at: "2026-07-30T00:00:00Z",
          updated_at: "2026-07-30T00:00:00Z",
          completed_at: null,
        };
      } else {
        run = {
          ...run,
          status: "running",
          active_task_id: handle.task_id,
          can_resume: false,
          resume_block_reason: null,
          error_message: null,
        };
      }
      generationTasks.set(handle.task_id, {
        handle,
        sequence: 0,
        channel: null,
        generationNumber,
      });
      return handle;
    }
    case "subscribe_generation_task": {
      const task = generationTasks.get(args.taskId);
      if (!task || task.handle.attempt !== args.attempt) {
        throw new Error("unexpected subscribe_generation_task input");
      }
      task.channel = args.onEvent;
      setTimeout(() => {
        emit(task, "started", startedData(task));
        const shotIndex = planningContentRunningScenario
          ? 2
          : task.generationNumber;
        if (task.generationNumber === 1) {
          checkpoints = [checkpoint(shotIndex)];
          run = {
            ...run,
            current_step_key: checkpoints[0].step_key,
            progress: {
              current: 1,
              total: 2,
              percent: 50,
              message: "已保存镜头 1",
              meta: null,
              updated_at: "2026-07-30T00:00:00Z",
            },
          };
        }
        emit(task, "progress", progressData(shotIndex));
        if (task.generationNumber === 1) {
          emit(task, "checkpoint", {
            checkpoint_key: checkpoints[0].step_key,
            resumable: true,
            summary: "已保存镜头 1",
          });
        }
      }, 0);
      return {
        subscription_id: `subscription-${task.generationNumber}`,
      };
    }
    case "cancel_generation_task": {
      const task = generationTasks.get(args.input?.task_id);
      if (!task || task.handle.attempt !== args.input?.attempt) {
        throw new Error("unexpected cancel_generation_task input");
      }
      cancelTask(task);
      return { ...task.handle, status: "cancelling" };
    }
    case "unsubscribe_generation_task":
      return null;
    case "update_chapter_video_segment_recipe": {
      const nextRecipe = args.recipe;
      if (args.promptId !== planId || !nextRecipe) {
        throw new Error("unexpected update_chapter_video_segment_recipe input");
      }
      if (
        referenceMediaSaveFailureScenario &&
        nextRecipe.segment_number === 1 &&
        nextRecipe.reference_media
      ) {
        throw new Error("参考媒体草稿保存失败测试");
      }
      if (parallelRecipeSavesScenario && nextRecipe.segment_number === 1) {
        await new Promise((resolve) => {
          releaseParallelFirstSave = resolve;
        });
      }
      if (
        crossModeContinuityScenario &&
        nextRecipe.continuity_source_video_id === sourceVideoId
      ) {
        await new Promise((resolve) => {
          releaseCrossModeSourceRecipeSave = resolve;
        });
      }
      updatePlanRecipe(nextRecipe);
      calls.push({
        command: "persisted_recipe_snapshot",
        target: nextRecipe.segment_number,
        modes: Object.fromEntries(
          plan.metadata_json.segment_recipes.map((current) => [
            current.segment_number,
            current.workflow_mode,
          ]),
        ),
      });
      if (parallelRecipeSavesScenario && nextRecipe.segment_number === 2) {
        if (!releaseParallelFirstSave) {
          throw new Error("first parallel recipe save did not start");
        }
        releaseParallelFirstSave();
        releaseParallelFirstSave = null;
      }
      return structuredClone(plan.metadata_json.segment_recipes);
    }
    case "update_chapter_video_prompt": {
      if (
        !storyboardPrompt ||
        args.promptId !== storyboardPrompt.id ||
        !["reject", "keep_downstream", "delete_downstream"].includes(
          args.input?.downstream_strategy,
        )
      ) {
        throw new Error("unexpected update_chapter_video_prompt input");
      }
      storyboardPrompt = {
        ...storyboardPrompt,
        visual_prompt:
          args.input?.visual_prompt ?? storyboardPrompt.visual_prompt,
        motion_prompt:
          args.input?.motion_prompt ?? storyboardPrompt.motion_prompt,
        negative_prompt:
          args.input?.negative_prompt ?? storyboardPrompt.negative_prompt,
      };
      return structuredClone(storyboardPrompt);
    }
    case "optimize_chapter_video_prompt": {
      if (
        !storyboardPrompt ||
        args.promptId !== storyboardPrompt.id ||
        !["reject", "keep_downstream", "delete_downstream"].includes(
          args.input?.downstream_strategy,
        )
      ) {
        throw new Error("unexpected optimize_chapter_video_prompt input");
      }
      await new Promise((resolve) => window.setTimeout(resolve, 2_000));
      storyboardPrompt = {
        ...storyboardPrompt,
        visual_prompt: `${storyboardPrompt.visual_prompt}\n优化要求：${args.input?.instruction}`,
      };
      return structuredClone(storyboardPrompt);
    }
    case "delete_chapter_video_prompt": {
      if (
        !storyboardPrompt ||
        args.input?.prompt_id !== storyboardPrompt.id ||
        !["reject", "keep_downstream", "delete_downstream"].includes(
          args.input?.downstream_strategy,
        )
      ) {
        throw new Error("unexpected delete_chapter_video_prompt input");
      }
      storyboardPrompt = null;
      return null;
    }
    default:
      throw new Error(`unexpected command: ${command}`);
  }
};

window.__TAURI_INTERNALS__ = {
  invoke,
  convertFileSrc: (path) => path,
  transformCallback: () => nextCallbackId++,
  unregisterCallback: () => undefined,
};
window.__A4N_VIDEO_MODE_E2E__ = {
  calls,
  publishReferenceMediaVideo: () => {
    referenceMediaVideoPublished = true;
  },
  setFinalPromptStatus: (status) => {
    if (!finalPrompt) throw new Error("final prompt missing");
    finalPrompt = { ...finalPrompt, status };
  },
};
await initializePortableStorage();

createRoot(document.getElementById("root")).render(
  <I18nProvider initialLocale="zh-CN">
    <ProgressTaskProvider>
      <div style={{ height: "100vh", padding: 16 }}>
        <ToastContainer />
        <ConfirmDialogHost />
        <PromptDialogHost />
        <ImageLightboxProvider>
          <VideoTab
            projectId={projectId}
            fixedChapter={{
              id: chapterId,
              chapter_number: 1,
              title: "第一章",
              content,
              status: "completed",
            }}
          />
        </ImageLightboxProvider>
      </div>
    </ProgressTaskProvider>
  </I18nProvider>,
);
