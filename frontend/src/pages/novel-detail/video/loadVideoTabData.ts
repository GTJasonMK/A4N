import {
  listVideoReferenceImages,
  type GeneratedImage,
} from "../../../api/image";
import { listWorkflowRuns, type WorkflowRun } from "../../../api/streaming";
import {
  loadVideoSegmentPlanDraftDetail,
  getChapterVideoTimeline,
  listChapterVideoPrompts,
  listVideoConfigs,
  listVideosByChapter,
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
import { VIDEO_ASPECT_RATIOS } from "./videoTabUtils";

export interface LoadedSegmentPlanDraft {
  workflowRunId: number;
  workflowStatus: string;
  resumeTaskId: string | null;
  resumeOptions: GeneratePromptOptions;
  workflowProgress: WorkflowRun["progress"];
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
  prompts: ChapterVideoPrompt[];
  videos: GeneratedVideo[];
  images: GeneratedImage[];
  activeVideoConfig: VideoGenConfig | null;
  timeline: VideoTimelineSummary;
}

const invalidSegmentPlanWorkflowInput = (
  runId: number,
  field: string,
): never => {
  throw new Error(
    `片段规划 workflow ${runId} 的原始输入 ${field} 无效，无法安全续跑`,
  );
};

const segmentPlanResumeOptions = (
  run: WorkflowRun,
  projectId: number,
  chapterId: number,
): GeneratePromptOptions => {
  const fingerprint =
    asJsonRecord(run.input_fingerprint_json) ??
    invalidSegmentPlanWorkflowInput(run.id, "input_fingerprint_json");
  if (fingerprint.schema_version !== 1) {
    invalidSegmentPlanWorkflowInput(run.id, "schema_version");
  }
  if (fingerprint.workflow_kind !== "video_segment_plan") {
    invalidSegmentPlanWorkflowInput(run.id, "workflow_kind");
  }
  if (
    fingerprint.project_id !== projectId ||
    fingerprint.chapter_id !== chapterId
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "project_id/chapter_id");
  }
  if (fingerprint.workflow_mode !== "shared_segment_plan") {
    invalidSegmentPlanWorkflowInput(run.id, "workflow_mode");
  }

  const options =
    asJsonRecord(fingerprint.options) ??
    invalidSegmentPlanWorkflowInput(run.id, "options");
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
  if (options.target_segment != null) {
    invalidSegmentPlanWorkflowInput(run.id, "options.target_segment");
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
    );
  }
  if (options.segment_recipe_fingerprint !== null) {
    invalidSegmentPlanWorkflowInput(
      run.id,
      "options.segment_recipe_fingerprint",
    );
  }

  const durationSecs = options.duration_secs;
  if (
    typeof durationSecs !== "number" ||
    !Number.isInteger(durationSecs) ||
    durationSecs < 1 ||
    durationSecs > 60
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs");
  }
  const resumeDurationSecs = durationSecs as number;
  const rawDurationOptions = options.duration_secs_options;
  if (!Array.isArray(rawDurationOptions)) {
    return invalidSegmentPlanWorkflowInput(
      run.id,
      "options.duration_secs_options",
    );
  }
  if (
    rawDurationOptions.some(
      (value) => typeof value !== "number" || !Number.isInteger(value),
    )
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options");
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
    invalidSegmentPlanWorkflowInput(run.id, "options.duration_secs_options");
  }

  const aspectRatio = options.aspect_ratio;
  if (
    typeof aspectRatio !== "string" ||
    !VIDEO_ASPECT_RATIOS.some((supported) => supported === aspectRatio)
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.aspect_ratio");
  }
  const resumeAspectRatio = aspectRatio as string;
  if (
    !Array.isArray(options.reference_image_ids) ||
    options.reference_image_ids.length !== 0
  ) {
    invalidSegmentPlanWorkflowInput(run.id, "options.reference_image_ids");
  }
  if (
    !Array.isArray(options.video_library_image_ids) ||
    options.video_library_image_ids.length !== 0
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
    target_narrative_unit_index:
      typeof targetNarrativeUnitIndex === "number"
        ? targetNarrativeUnitIndex
        : undefined,
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
    resumable_only: false,
    limit: 1,
  });
  const run = runs[0];
  if (!run || (run.status !== "running" && !run.can_resume)) return null;
  const [detail, resumeTaskId] = await Promise.all([
    loadVideoSegmentPlanDraftDetail(projectId, chapterId, run.id),
    run.status === "running"
      ? Promise.resolve(null)
      : workflowResumeTaskId(projectId, run.id),
  ]);
  return loadedVideoSegmentPlanDraftFromDetail(
    detail,
    projectId,
    chapterId,
    resumeTaskId,
  );
};

const workflowResumeTaskId = async (
  projectId: number,
  workflowRunId: number,
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
  throw new Error(`片段规划 workflow ${workflowRunId} 缺少可用于续跑的生成任务`);
};

const loadedVideoSegmentPlanDraftFromDetail = (
  detail: VideoSegmentPlanDraftDetail,
  projectId: number,
  chapterId: number,
  resumeTaskId: string | null,
): LoadedSegmentPlanDraft => {
  const run = detail.run;
  if (
    run.project_id !== projectId ||
    run.chapter_id !== chapterId ||
    run.workflow_kind !== "video_segment_plan"
  ) {
    throw new Error(`片段规划 workflow ${detail.run.id} 与当前章节不一致`);
  }
  const resumeOptions = segmentPlanResumeOptions(run, projectId, chapterId);
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
  const [prompts, videos, images, configs, timeline] = await Promise.all([
    listChapterVideoPrompts(projectId, chapter.id),
    listVideosByChapter(projectId, chapter.id),
    listVideoReferenceImages(projectId, chapter.id, uniqueImageProjectIds),
    listVideoConfigs(),
    getChapterVideoTimeline({ project_id: projectId, chapter_id: chapter.id }),
  ]);

  return {
    prompts,
    videos,
    images,
    activeVideoConfig: configs.find((config) => config.is_default) ?? null,
    timeline,
  };
};
