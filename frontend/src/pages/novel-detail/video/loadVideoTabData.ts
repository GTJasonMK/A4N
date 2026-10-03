import {
  listVideoReferenceImages,
  type GeneratedImage,
} from "../../../api/image";
import {
  listWorkflowRuns,
  type WorkflowRunSummary,
} from "../../../api/streaming";
import {
  getChapterVideoTimeline,
  loadVideoSegmentPlanDraftDetail,
  listChapterVideoPrompts,
  listVideoConfigs,
  listVideosByChapter,
  VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
  type ChapterVideoPrompt,
  type GeneratePromptOptions,
  type GeneratedVideo,
  type VideoGenConfig,
  type VideoSegmentExecutionRecipe,
  type VideoSegmentPlanDraftDetail,
  type VideoTimelineSummary,
} from "../../../api/video";
import type { ChapterRow } from "../shared";
import type { GenerationTaskSummary } from "../../../api/generated/generationContracts";
import { generationClient } from "../../../api/generationTasks";
import { asJsonRecord } from "../../../utils/jsonRecord";
import { VIDEO_ASPECT_RATIOS } from "./videoTabConstants";
import type { MessageTranslator } from "../../../i18n";

export interface LoadedSegmentPlanDraft {
  workflowRunId: number;
  workflowStatus: string;
  resumeTaskId: string | null;
  resumeTaskError?: unknown;
  resumeOptions: GeneratePromptOptions;
  workflowProgress: WorkflowRunSummary["progress"];
  segments: Record<string, unknown>[];
  recipes: Map<number, VideoSegmentExecutionRecipe>;
  completedShotCount: number;
  totalShotCount: number;
}

export const loadedSegmentPlanDraftIsActive = (
  draft: LoadedSegmentPlanDraft | null,
  generationActive: boolean,
): boolean => draft?.workflowStatus === "running" || generationActive;

interface VideoTabData {
  prompts?: ChapterVideoPrompt[];
  videos?: GeneratedVideo[];
  images?: GeneratedImage[];
  activeVideoConfig?: VideoGenConfig | null;
  timeline?: VideoTimelineSummary;
  errors: Array<{ key: "prompts" | "videos" | "images" | "configs" | "timeline"; reason: unknown }>;
}

const invalidSegmentPlanWorkflowInput = (
  runId: number,
  field: string,
  t: MessageTranslator,
): never => {
  throw new Error(
    t("video.chapter.segmentPlanWorkflowInputInvalid", { runId, field }),
  );
};

const segmentPlanResumeOptions = (
  run: WorkflowRunSummary,
  projectId: number,
  chapterId: number,
  t: MessageTranslator,
): GeneratePromptOptions => {
  const fingerprint =
    asJsonRecord(run.input_fingerprint_json) ??
    invalidSegmentPlanWorkflowInput(run.id, "input_fingerprint_json", t);
  if (fingerprint.schema_version !== 1) {
    invalidSegmentPlanWorkflowInput(run.id, "schema_version", t);
  }
  if (fingerprint.workflow_kind !== "video_segment_plan") {
    invalidSegmentPlanWorkflowInput(run.id, "workflow_kind", t);
  }
  if (
    fingerprint.project_id !== projectId ||
    fingerprint.chapter_id !== chapterId
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "project_id/chapter_id", t);
  }
  if (fingerprint.workflow_mode !== VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN) {
    invalidSegmentPlanWorkflowInput(run.id, "workflow_mode", t);
  }

  const options =
    asJsonRecord(fingerprint.options) ??
    invalidSegmentPlanWorkflowInput(run.id, "options", t);
  const style = options.style;
  if (typeof style !== "string" || !style || style.trim() !== style) {
    invalidSegmentPlanWorkflowInput(run.id, "options.style", t);
  }
  const resumeStyle = style as string;
  const promptOutputLanguage = options.prompt_output_language;
  if (promptOutputLanguage !== "zh_cn" && promptOutputLanguage !== "en") {
    invalidSegmentPlanWorkflowInput(run.id, "options.prompt_output_language", t);
  }
  const resumePromptOutputLanguage = promptOutputLanguage as "zh_cn" | "en";
  if (options.prompt_batch_size !== 1) {
    invalidSegmentPlanWorkflowInput(run.id, "options.prompt_batch_size", t);
  }
  if (options.segment_plan_prompt_id !== null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.segment_plan_prompt_id", t);
  }
  if (options.target_shot_number !== null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.target_shot_number", t);
  }
  if (options.target_segment != null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.target_segment", t);
  }
  const targetNarrativeUnitIndex = options.target_narrative_unit_index;
  if (
    targetNarrativeUnitIndex != null &&
    (typeof targetNarrativeUnitIndex !== "number" ||
      !Number.isInteger(targetNarrativeUnitIndex) ||
      targetNarrativeUnitIndex <= 0)
  ) {
    invalidSegmentPlanWorkflowInput(
      run.id,
      "options.target_narrative_unit_index",
      t,
    );
  }
  if (options.segment_recipe_fingerprint !== null) {
    invalidSegmentPlanWorkflowInput(
      run.id,
      "options.segment_recipe_fingerprint",
      t,
    );
  }

  const durationSecs = options.duration_secs;
  if (
    typeof durationSecs !== "number" ||
    !Number.isInteger(durationSecs) ||
    durationSecs < 1 ||
    durationSecs > 60
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs", t);
  }
  const resumeDurationSecs = durationSecs as number;
  const rawDurationOptions = options.duration_secs_options;
  if (!Array.isArray(rawDurationOptions)) {
    return invalidSegmentPlanWorkflowInput(
      run.id,
      "options.duration_secs_options",
      t,
    );
  }
  if (
    rawDurationOptions.some(
      (value) => typeof value !== "number" || !Number.isInteger(value),
    )
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options", t);
  }
  const durationOptions = rawDurationOptions as number[];
  if (
    durationOptions.length === 0 ||
    durationOptions.length > 12 ||
    durationOptions.some((value) => value < 1 || value > 60) ||
    !durationOptions.includes(resumeDurationSecs) ||
    durationOptions.some(
      (value, index) => index > 0 && value <= durationOptions[index - 1],
    )
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options", t);
  }

  const aspectRatio = options.aspect_ratio;
  if (
    typeof aspectRatio !== "string" ||
    !VIDEO_ASPECT_RATIOS.some((supported) => supported === aspectRatio)
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.aspect_ratio", t);
  }
  const resumeAspectRatio = aspectRatio as string;
  if (
    !Array.isArray(options.reference_image_ids) ||
    options.reference_image_ids.length !== 0
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.reference_image_ids", t);
  }
  if (
    !Array.isArray(options.video_library_image_ids) ||
    options.video_library_image_ids.length !== 0
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.video_library_image_ids", t);
  }
  if (typeof options.regenerate_existing !== "boolean") {
    invalidSegmentPlanWorkflowInput(run.id, "options.regenerate_existing", t);
  }
  const regenerateExisting = options.regenerate_existing as boolean;

  return {
    workflow_mode: VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
    style: resumeStyle,
    prompt_output_language: resumePromptOutputLanguage,
    prompt_batch_size: 1,
    duration_secs: resumeDurationSecs,
    duration_secs_options: durationOptions,
    aspect_ratio: resumeAspectRatio,
    reference_image_ids: [],
    video_library_image_ids: [],
    regenerate_existing: regenerateExisting,
    target_narrative_unit_index:
      typeof targetNarrativeUnitIndex === "number"
        ? targetNarrativeUnitIndex
        : undefined,
  };
};

export const loadVideoSegmentPlanDraft = async (
  projectId: number,
  chapterId: number,
  t: MessageTranslator,
): Promise<LoadedSegmentPlanDraft | null> => {
  const runs = await listWorkflowRuns({
    project_id: projectId,
    chapter_id: chapterId,
    workflow_kind: "video_segment_plan",
    resumable_only: false,
    limit: 200,
  });
  const run = runs.find(
    (candidate) =>
      candidate.mode === VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN &&
      (candidate.status === "running" || candidate.can_resume),
  );
  if (!run) return null;
  const [detailResult, resumeResult] = await Promise.allSettled([
    loadVideoSegmentPlanDraftDetail(projectId, chapterId, run.id),
    run.status === "running"
      ? Promise.resolve(null)
      : workflowResumeTaskId(projectId, run.id, t),
  ]);
  if (detailResult.status === "rejected") throw detailResult.reason;
  const draft = loadedVideoSegmentPlanDraftFromDetail(
    detailResult.value,
    projectId,
    chapterId,
    resumeResult.status === "fulfilled" ? resumeResult.value : null,
    t,
  );
  if (resumeResult.status === "rejected") draft.resumeTaskError = resumeResult.reason;
  return draft;
};

const workflowResumeTaskId = async (
  projectId: number,
  workflowRunId: number,
  t: MessageTranslator,
): Promise<string> => {
  let cursor: string | null = null;
  do {
    const page = await generationClient.listHistory({
      project_id: projectId,
      limit: 100,
      cursor,
    });
    const task = page.items.find(
      (item) =>
        item.started.workflow_run_id === workflowRunId &&
        item.started.root_task_id === item.handle.task_id,
    );
    if (task) return task.handle.task_id;
    cursor = page.next_cursor ?? null;
  } while (cursor);
  throw new Error(
    t("video.chapter.segmentPlanWorkflowTaskMissing", { runId: workflowRunId }),
  );
};

const loadedVideoSegmentPlanDraftFromDetail = (
  detail: VideoSegmentPlanDraftDetail,
  projectId: number,
  chapterId: number,
  resumeTaskId: string | null,
  t: MessageTranslator,
): LoadedSegmentPlanDraft => {
  const run = detail.run;
  if (
    run.project_id !== projectId ||
    run.chapter_id !== chapterId ||
    run.workflow_kind !== "video_segment_plan"
  ) {
    throw new Error(
      t("video.chapter.segmentPlanWrongChapter", { runId: detail.run.id }),
    );
  }
  const resumeOptions = segmentPlanResumeOptions(run, projectId, chapterId, t);
  return {
    workflowRunId: run.id,
    workflowStatus: run.status,
    resumeTaskId,
    resumeOptions,
    workflowProgress: run.progress,
    segments: detail.segments,
    recipes: new Map(
      detail.recipes.map((recipe) => [recipe.segment_number, recipe]),
    ),
    completedShotCount: detail.completed_shot_count,
    totalShotCount: detail.total_shot_count,
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
  return (
    tasks
      .filter(
        (task) =>
          task.started.target?.kind === "video_prompt_segment_plan" &&
          task.started.target.id === targetId,
      )
      .sort((left, right) => right.updated_at_ms - left.updated_at_ms)[0] ??
    null
  );
};

export const loadVideoTabData = async (
  projectId: number,
  chapter: ChapterRow,
  imageProjectIds: number[] = [projectId],
): Promise<VideoTabData> => {
  const uniqueImageProjectIds = Array.from(
    new Set(imageProjectIds.filter((id) => Number.isFinite(id) && id > 0)),
  );
  const [prompts, videos, images, configs, timeline] = await Promise.allSettled([
    listChapterVideoPrompts(projectId, chapter.id),
    listVideosByChapter(projectId, chapter.id),
    listVideoReferenceImages(projectId, chapter.id, uniqueImageProjectIds),
    listVideoConfigs(),
    getChapterVideoTimeline({ project_id: projectId, chapter_id: chapter.id }),
  ]);

  return {
    prompts: prompts.status === "fulfilled" ? prompts.value : undefined,
    videos: videos.status === "fulfilled" ? videos.value : undefined,
    images: images.status === "fulfilled" ? images.value : undefined,
    activeVideoConfig: configs.status === "fulfilled"
      ? configs.value.find((config) => config.is_default) ?? null
      : undefined,
    timeline: timeline.status === "fulfilled" ? timeline.value : undefined,
    errors: [
      ...(prompts.status === "rejected" ? [{ key: "prompts" as const, reason: prompts.reason }] : []),
      ...(videos.status === "rejected" ? [{ key: "videos" as const, reason: videos.reason }] : []),
      ...(images.status === "rejected" ? [{ key: "images" as const, reason: images.reason }] : []),
      ...(configs.status === "rejected" ? [{ key: "configs" as const, reason: configs.reason }] : []),
      ...(timeline.status === "rejected" ? [{ key: "timeline" as const, reason: timeline.reason }] : []),
    ],
  };
};
