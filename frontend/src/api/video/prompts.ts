import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { GenerationTaskHandle } from "../generated/generationContracts";
import { idNum, type ApiId as Id } from "../shared";
import { startVideoGeneration } from "./generation";
import type { WorkflowRun } from "../streaming";
import type {
  ChapterVideoPrompt,
  DownstreamConflictStrategy,
  PromptOutputLanguage,
  VideoWorkflowMode,
} from "./types";

export interface GeneratePromptOptions {
  workflow_mode: VideoWorkflowMode;
  segment_plan_prompt_id?: number;
  style: string;
  prompt_output_language?: PromptOutputLanguage;
  prompt_batch_size: number;
  target_shot_number?: number;
  duration_secs: number;
  duration_secs_options?: number[];
  aspect_ratio: string;
  reference_image_ids: number[];
  video_library_image_ids?: number[];
  regenerate_existing?: boolean;
}

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

interface UpdateChapterVideoPromptInput {
  visual_prompt?: string | null;
  motion_prompt?: string | null;
  negative_prompt?: string | null;
  downstream_strategy?: DownstreamConflictStrategy;
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

interface OptimizeChapterVideoPromptInput {
  instruction: string;
  downstream_strategy?: DownstreamConflictStrategy;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问视频提示词");
  return id;
};

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
): Promise<GenerationTaskHandle> {
  return startVideoGeneration({
    project_id: idNum(projectId, "项目 ID"),
    operation: {
      kind: "prompt_segment_plan",
      chapter_id: idNum(chapterId, "章节 ID"),
      options,
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
  downstreamStrategy: DownstreamConflictStrategy = "reject",
): Promise<void> {
  return apiInvoke<void>("delete_chapter_video_prompt", {
    userId: currentUserId(),
    promptId: idNum(promptId, "视频提示词 ID"),
    downstreamStrategy,
  });
}
