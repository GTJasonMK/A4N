import { listVideoReferenceImages, type GeneratedImage } from "../../../api/image";
import {
  listWorkflowRuns,
  type WorkflowRun,
  type WorkflowRunDetail,
} from "../../../api/streaming";
import {
  loadVideoSegmentPlanDraft as loadVideoSegmentPlanDraftDetail,
  listChapterVideoPrompts,
  listEffectiveVideoConfigs,
  listVideosByChapter,
  type ChapterVideoPrompt,
  type GeneratePromptOptions,
  type GeneratedVideo,
  type VideoGenConfig,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import type { GenerationTaskSummary } from "../../../api/generated/generationContracts";
import { asJsonRecord } from "../../../utils/jsonRecord";
import {
  segmentPlanDraftSnapshotFromCheckpoints,
  type SegmentPlanDraftSnapshot,
  VIDEO_ASPECT_RATIOS,
} from "./videoTabUtils";

export interface LoadedSegmentPlanDraft extends SegmentPlanDraftSnapshot {
  workflowRunId: number;
  workflowStatus: string;
  resumable: boolean;
  resumeOptions: GeneratePromptOptions;
  workflowProgress: WorkflowRun["progress"];
}

interface VideoTabData {
  prompts: ChapterVideoPrompt[];
  videos: GeneratedVideo[];
  images: GeneratedImage[];
  activeVideoConfig: VideoGenConfig | null;
}

const invalidSegmentPlanWorkflowInput = (runId: number, field: string): never => {
  throw new Error(`片段规划 workflow ${runId} 的原始输入 ${field} 无效，无法安全续跑`);
};

const segmentPlanResumeOptions = (
  run: WorkflowRun,
  projectId: number,
  chapterId: number,
): GeneratePromptOptions => {
  const fingerprint = asJsonRecord(run.input_fingerprint_json)
    ?? invalidSegmentPlanWorkflowInput(run.id, "input_fingerprint_json");
  if (fingerprint.schema_version !== 1) {
    invalidSegmentPlanWorkflowInput(run.id, "schema_version");
  }
  if (fingerprint.workflow_kind !== "video_segment_plan") {
    invalidSegmentPlanWorkflowInput(run.id, "workflow_kind");
  }
  if (fingerprint.project_id !== projectId || fingerprint.chapter_id !== chapterId) {
    invalidSegmentPlanWorkflowInput(run.id, "project_id/chapter_id");
  }
  if (fingerprint.workflow_mode !== "shared_segment_plan") {
    invalidSegmentPlanWorkflowInput(run.id, "workflow_mode");
  }

  const options = asJsonRecord(fingerprint.options)
    ?? invalidSegmentPlanWorkflowInput(run.id, "options");
  const style = options.style;
  if (typeof style !== "string" || !style || style.trim() !== style) {
    invalidSegmentPlanWorkflowInput(run.id, "options.style");
  }
  const resumeStyle = style as string;
  const promptOutputLanguage = options.prompt_output_language;
  if (promptOutputLanguage !== "zh_cn" && promptOutputLanguage !== "en") {
    invalidSegmentPlanWorkflowInput(run.id, "options.prompt_output_language");
  }
  const resumePromptOutputLanguage = promptOutputLanguage as "zh_cn" | "en";
  if (options.prompt_batch_size !== 1) {
    invalidSegmentPlanWorkflowInput(run.id, "options.prompt_batch_size");
  }
  if (options.segment_plan_prompt_id !== null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.segment_plan_prompt_id");
  }
  if (options.target_shot_number !== null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.target_shot_number");
  }
  if (options.segment_recipe_fingerprint !== null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.segment_recipe_fingerprint");
  }

  const durationSecs = options.duration_secs;
  if (
    typeof durationSecs !== "number"
    || !Number.isInteger(durationSecs)
    || durationSecs < 1
    || durationSecs > 60
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs");
  }
  const resumeDurationSecs = durationSecs as number;
  const rawDurationOptions = options.duration_secs_options;
  if (!Array.isArray(rawDurationOptions)) {
    return invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options");
  }
  if (rawDurationOptions.some((value) => typeof value !== "number" || !Number.isInteger(value))) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options");
  }
  const durationOptions = rawDurationOptions as number[];
  if (
    durationOptions.length === 0
    || durationOptions.length > 12
    || durationOptions.some((value) => value < 1 || value > 60)
    || !durationOptions.includes(resumeDurationSecs)
    || durationOptions.some((value, index) => index > 0 && value <= durationOptions[index - 1])
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options");
  }

  const aspectRatio = options.aspect_ratio;
  if (
    typeof aspectRatio !== "string"
    || !VIDEO_ASPECT_RATIOS.some((supported) => supported === aspectRatio)
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.aspect_ratio");
  }
  const resumeAspectRatio = aspectRatio as string;
  if (!Array.isArray(options.reference_image_ids) || options.reference_image_ids.length !== 0) {
    invalidSegmentPlanWorkflowInput(run.id, "options.reference_image_ids");
  }
  if (
    !Array.isArray(options.video_library_image_ids)
    || options.video_library_image_ids.length !== 0
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.video_library_image_ids");
  }
  if (typeof options.regenerate_existing !== "boolean") {
    invalidSegmentPlanWorkflowInput(run.id, "options.regenerate_existing");
  }
  const regenerateExisting = options.regenerate_existing as boolean;

  return {
    workflow_mode: "shared_segment_plan",
    style: resumeStyle,
    prompt_output_language: resumePromptOutputLanguage,
    prompt_batch_size: 1,
    duration_secs: resumeDurationSecs,
    duration_secs_options: durationOptions,
    aspect_ratio: resumeAspectRatio,
    reference_image_ids: [],
    video_library_image_ids: [],
    regenerate_existing: regenerateExisting,
  };
};

export const loadVideoSegmentPlanDraft = async (
  projectId: number,
  chapterId: number,
): Promise<LoadedSegmentPlanDraft | null> => {
  const runs = await listWorkflowRuns({
    project_id: projectId,
    chapter_id: chapterId,
    workflow_kind: "video_segment_plan",
    limit: 1,
  });
  const run = runs[0];
  if (!run || run.status === "superseded") return null;
  const detail = await loadVideoSegmentPlanDraftDetail(projectId, chapterId, run.id);
  return loadedVideoSegmentPlanDraftFromDetail(detail, projectId, chapterId);
};

export const loadedVideoSegmentPlanDraftFromDetail = (
  detail: WorkflowRunDetail,
  projectId: number,
  chapterId: number,
): LoadedSegmentPlanDraft | null => {
  const run = detail.run;
  if (
    run.project_id !== projectId ||
    run.chapter_id !== chapterId ||
    run.workflow_kind !== "video_segment_plan"
  ) {
    throw new Error(`片段规划 workflow ${detail.run.id} 与当前章节不一致`);
  }
  const snapshot: SegmentPlanDraftSnapshot | null =
    segmentPlanDraftSnapshotFromCheckpoints(detail.checkpoints);
  if (!snapshot) return null;
  const resumeOptions = segmentPlanResumeOptions(run, projectId, chapterId);
  return {
    workflowRunId: run.id,
    workflowStatus: run.status,
    resumable: run.status === "running" || run.can_resume === true,
    resumeOptions,
    workflowProgress: run.progress,
    segments: snapshot.segments,
    recipes: snapshot.recipes,
    completedShotCount: snapshot.completedShotCount,
    totalShotCount: snapshot.totalShotCount,
  };
};

export const videoSegmentPlanGenerationTargetId = (
  projectId: number,
  chapterId: number,
): string => `${projectId}:chapter:${chapterId}:prompt_segment_plan:shared`;

export const activeVideoSegmentPlanTask = (
  tasks: GenerationTaskSummary[],
  projectId: number,
  chapterId: number,
): GenerationTaskSummary | null => {
  const targetId = videoSegmentPlanGenerationTargetId(projectId, chapterId);
  return tasks
    .filter(
      (task) =>
        task.started.target?.kind === "video_prompt_segment_plan" &&
        task.started.target.id === targetId,
    )
    .sort((left, right) => right.updated_at_ms - left.updated_at_ms)[0] ?? null;
};

export const loadVideoTabData = async (
  projectId: number,
  chapter: ChapterRow,
  imageProjectIds: number[] = [projectId],
): Promise<VideoTabData> => {
  const uniqueImageProjectIds = Array.from(
    new Set(imageProjectIds.filter((id) => Number.isFinite(id) && id > 0)),
  );
  const [prompts, videos, images, configs] = await Promise.all([
    listChapterVideoPrompts(projectId, chapter.id),
    listVideosByChapter(projectId, chapter.id),
    listVideoReferenceImages(projectId, chapter.id, uniqueImageProjectIds),
    listEffectiveVideoConfigs(),
  ]);

  return {
    prompts,
    videos,
    images,
    activeVideoConfig: configs.find((config) => config.is_default) ?? null,
  };
};
