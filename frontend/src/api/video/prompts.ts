import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
import type { GenerationTaskHandle } from "../generated/generationContracts";
import type { VideoInsertedSegmentGenerationOptions } from "../generated/generationContracts";
import type {
  CommitInsertedSegmentDraftInput,
  DeleteChapterVideoPromptInput,
  DeleteInsertedSegmentInput,
  DownstreamConflictStrategy,
  GetVideoTimelineInput,
  MoveInsertedSegmentInput,
  OptimizeChapterVideoPromptInput,
  UpdateChapterVideoPromptInput,
  VideoTimelineItemSummary,
  VideoTimelineSummary,
} from "../generated/host";
import { idNum, type ApiId as Id } from "../shared";
import { startVideoGeneration } from "./generation";
import type { WorkflowRun } from "../streaming";
import type {
  ChapterVideoPrompt,
  PromptOutputLanguage,
  VideoWorkflowMode,
} from "./types";

export interface GeneratePromptOptions {
  workflow_mode: VideoWorkflowMode;
  segment_plan_prompt_id?: number;
  style: string;
  prompt_output_language?: PromptOutputLanguage;
  prompt_batch_size: number;
  target_segment?: {
    source_shot_id: string;
    segment_index_in_shot: number;
  };
  target_narrative_unit_index?: number;
  duration_secs: number;
  duration_secs_options?: number[];
  aspect_ratio: string;
  reference_image_ids: number[];
  video_library_image_ids?: number[];
  regenerate_existing?: boolean;
}

export type InsertedSegmentGenerationOptions =
  VideoInsertedSegmentGenerationOptions;
export type {
  CommitInsertedSegmentDraftInput,
  DeleteInsertedSegmentInput,
  GetVideoTimelineInput,
  MoveInsertedSegmentInput,
  VideoTimelineItemSummary,
  VideoTimelineSummary,
};

export type SegmentGuidanceMode =
  "system_frames" | "keyframes" | "none" | "continuity";

type SegmentWorkflowMode =
  | "boundary_frames"
  | "first_frame_chain"
  | "storyboard"
  | "reference_media"
  | "chat_video";

interface SegmentReferenceMediaBinding {
  url: string;
  usage: string;
  upload_sha256: string | null;
  upload_file_name: string | null;
  upload_mime_type: string | null;
}

export interface SegmentReferenceMediaRecipe {
  prompt: string;
  image_rows: SegmentReferenceMediaBinding[];
  video_rows: SegmentReferenceMediaBinding[];
  audio_rows: SegmentReferenceMediaBinding[];
}

export interface VideoSegmentExecutionRecipe {
  schema_version: "video_segment_execution_recipe_v1";
  segment_number: number;
  workflow_mode: SegmentWorkflowMode;
  style: string;
  prompt_output_language: PromptOutputLanguage;
  duration_secs: number;
  duration_secs_options: number[];
  aspect_ratio: string;
  reference_image_ids: number[];
  video_library_image_ids: number[];
  guidance_mode: SegmentGuidanceMode;
  guidance_reference_image_ids: number[];
  continuity_source_video_id: number | null;
  reference_media: SegmentReferenceMediaRecipe | null;
  adopted_video_id: number | null;
}

interface UpdateVideoSegmentPlanDraftRecipeInput {
  project_id: number;
  chapter_id: number;
  workflow_run_id: number;
  source_shot_id: string;
  segment_index_in_shot: number;
  recipe: VideoSegmentExecutionRecipe;
}

export interface VideoSegmentPlanDraftDetail {
  run: WorkflowRun;
  segments: Record<string, unknown>[];
  recipes: VideoSegmentExecutionRecipe[];
  completed_shot_count: number;
  total_shot_count: number;
}

export async function startChapterVideoFramePrompts(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
): Promise<GenerationTaskHandle> {
  return startVideoGeneration({
    project_id: idNum(projectId, "项目 ID"),
    operation: {
      kind: "prompt_frames",
      chapter_id: idNum(chapterId, "章节 ID"),
      options,
    },
  });
}

export async function startChapterVideoSegmentPlan(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
  retryOfTaskId?: string | null,
): Promise<GenerationTaskHandle> {
  const input = {
    project_id: idNum(projectId, "项目 ID"),
    operation: {
      kind: "prompt_segment_plan" as const,
      chapter_id: idNum(chapterId, "章节 ID"),
      options,
    },
  };
  return retryOfTaskId == null
    ? startVideoGeneration(input)
    : startVideoGeneration(input, retryOfTaskId);
}

export async function startChapterVideoInsertedSegmentDraft(
  projectId: Id,
  chapterId: Id,
  options: InsertedSegmentGenerationOptions,
  retryOfTaskId?: string | null,
): Promise<GenerationTaskHandle> {
  const input = {
    project_id: idNum(projectId, "项目 ID"),
    operation: {
      kind: "prompt_inserted_segment" as const,
      chapter_id: idNum(chapterId, "章节 ID"),
      options,
    },
  };
  return retryOfTaskId == null
    ? startVideoGeneration(input)
    : startVideoGeneration(input, retryOfTaskId);
}

export async function getChapterVideoTimeline(
  input: GetVideoTimelineInput,
): Promise<VideoTimelineSummary> {
  return apiInvoke<VideoTimelineSummary>("get_chapter_video_timeline", {
    userId: currentUserId(),
    input: {
      project_id: idNum(input.project_id, "项目 ID"),
      chapter_id: idNum(input.chapter_id, "章节 ID"),
    },
  });
}

export async function commitInsertedSegmentDraft(
  input: CommitInsertedSegmentDraftInput,
): Promise<VideoTimelineSummary> {
  return apiInvoke<VideoTimelineSummary>("commit_inserted_segment_draft", {
    userId: currentUserId(),
    input: {
      timeline_id: idNum(input.timeline_id, "时间线 ID"),
      expected_timeline_revision: idNum(
        input.expected_timeline_revision,
        "时间线 revision",
      ),
      workflow_run_id: idNum(input.workflow_run_id, "草稿 workflow ID"),
    },
  });
}

export async function moveInsertedSegment(
  input: MoveInsertedSegmentInput,
): Promise<VideoTimelineSummary> {
  return apiInvoke<VideoTimelineSummary>("move_inserted_segment", {
    userId: currentUserId(),
    input: {
      timeline_id: idNum(input.timeline_id, "时间线 ID"),
      expected_timeline_revision: idNum(
        input.expected_timeline_revision,
        "时间线 revision",
      ),
      timeline_item_id: idNum(input.timeline_item_id, "时间线项 ID"),
      previous_item_id:
        input.previous_item_id == null ? null : idNum(input.previous_item_id),
      next_item_id:
        input.next_item_id == null ? null : idNum(input.next_item_id),
    },
  });
}

export async function deleteInsertedSegment(
  input: DeleteInsertedSegmentInput,
): Promise<VideoTimelineSummary> {
  return apiInvoke<VideoTimelineSummary>("delete_inserted_segment", {
    userId: currentUserId(),
    input: {
      timeline_id: idNum(input.timeline_id, "时间线 ID"),
      expected_timeline_revision: idNum(
        input.expected_timeline_revision,
        "时间线 revision",
      ),
      timeline_item_id: idNum(input.timeline_item_id, "时间线项 ID"),
    },
  });
}

export async function startChapterVideoPrompts(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
): Promise<GenerationTaskHandle> {
  return startVideoGeneration({
    project_id: idNum(projectId, "项目 ID"),
    operation: {
      kind: "prompt_videos",
      chapter_id: idNum(chapterId, "章节 ID"),
      options,
    },
  });
}

export async function listChapterVideoPrompts(
  projectId: Id,
  chapterId: Id,
): Promise<ChapterVideoPrompt[]> {
  return apiInvoke<ChapterVideoPrompt[]>("list_chapter_video_prompts", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
  });
}

export async function loadVideoSegmentPlanDraftDetail(
  projectId: Id,
  chapterId: Id,
  workflowRunId: Id,
): Promise<VideoSegmentPlanDraftDetail> {
  return apiInvoke<VideoSegmentPlanDraftDetail>(
    "load_video_segment_plan_draft",
    {
      userId: currentUserId(),
      input: {
        project_id: idNum(projectId, "项目 ID"),
        chapter_id: idNum(chapterId, "章节 ID"),
        workflow_run_id: idNum(workflowRunId, "片段规划 workflow ID"),
      },
    },
  );
}

export async function updateVideoSegmentPlanDraftRecipe(
  input: UpdateVideoSegmentPlanDraftRecipeInput,
): Promise<void> {
  return apiInvoke<void>("update_video_segment_plan_draft_recipe", {
    userId: currentUserId(),
    input,
  });
}

export async function updateChapterVideoSegmentRecipe(
  promptId: Id,
  recipe: VideoSegmentExecutionRecipe,
): Promise<VideoSegmentExecutionRecipe[]> {
  return apiInvoke<VideoSegmentExecutionRecipe[]>(
    "update_chapter_video_segment_recipe",
    {
      userId: currentUserId(),
      promptId: idNum(promptId, "视频片段规划 ID"),
      recipe,
    },
  );
}

export async function updateChapterVideoPrompt(
  promptId: Id,
  input: UpdateChapterVideoPromptInput,
): Promise<ChapterVideoPrompt> {
  return apiInvoke<ChapterVideoPrompt>("update_chapter_video_prompt", {
    userId: currentUserId(),
    promptId: idNum(promptId, "视频提示词 ID"),
    input,
  });
}

export async function optimizeChapterVideoPrompt(
  promptId: Id,
  input: OptimizeChapterVideoPromptInput,
): Promise<ChapterVideoPrompt> {
  return apiInvoke<ChapterVideoPrompt>("optimize_chapter_video_prompt", {
    userId: currentUserId(),
    promptId: idNum(promptId, "视频提示词 ID"),
    input,
  });
}

export async function deleteChapterVideoPrompt(
  promptId: Id,
  downstreamStrategy: DownstreamConflictStrategy,
): Promise<void> {
  return apiInvoke<void>("delete_chapter_video_prompt", {
    userId: currentUserId(),
    input: {
      prompt_id: idNum(promptId, "视频提示词 ID"),
      downstream_strategy: downstreamStrategy,
    } satisfies DeleteChapterVideoPromptInput,
  });
}
