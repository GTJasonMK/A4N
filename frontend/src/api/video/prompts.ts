import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import type { ModelRequestPreview } from "../requestPreview";
import { idNum, type ApiId as Id } from "../shared";
import type {
  ChapterVideoPrompt,
  DownstreamConflictStrategy,
  PromptOutputLanguage,
  VideoWorkflowMode,
} from "./types";

interface GeneratePromptOptions {
  workflow_mode: VideoWorkflowMode;
  style: string;
  prompt_output_language?: PromptOutputLanguage;
  prompt_batch_size: number;
  duration_secs: number;
  duration_secs_options?: number[];
  aspect_ratio: string;
  reference_image_ids: number[];
  video_library_image_ids?: number[];
  regenerate_existing?: boolean;
}

interface UpdateChapterVideoPromptInput {
  visual_prompt?: string | null;
  motion_prompt?: string | null;
  negative_prompt?: string | null;
  duration_secs?: number | null;
  reference_image_ids_json?: unknown | null;
  downstream_strategy?: DownstreamConflictStrategy;
}

interface OptimizeChapterVideoPromptInput {
  instruction: string;
  downstream_strategy?: DownstreamConflictStrategy;
}

export interface DeleteChapterVideoPromptGroupResult {
  deleted_prompt_count: number;
  deleted_image_count: number;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问视频提示词");
  return id;
};

export async function generateChapterVideoFramePrompts(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
  taskId?: string | null,
): Promise<ChapterVideoPrompt[]> {
  return apiInvoke<ChapterVideoPrompt[]>("generate_chapter_video_frame_prompts", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    options,
    taskId: taskId ?? null,
  });
}

export async function generateChapterVideoSegmentPlan(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
  taskId?: string | null,
): Promise<ChapterVideoPrompt[]> {
  return apiInvoke<ChapterVideoPrompt[]>("generate_chapter_video_segment_plan", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    options,
    taskId: taskId ?? null,
  });
}

export async function generateChapterVideoShotAnalysis(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
  taskId?: string | null,
): Promise<ChapterVideoPrompt[]> {
  return apiInvoke<ChapterVideoPrompt[]>("generate_chapter_video_shot_analysis", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    options,
    taskId: taskId ?? null,
  });
}

export async function generateChapterVideoPrompts(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
  taskId?: string | null,
): Promise<ChapterVideoPrompt[]> {
  return apiInvoke<ChapterVideoPrompt[]>("generate_chapter_video_prompts", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    options,
    taskId: taskId ?? null,
  });
}

export async function previewChapterVideoFramePromptRequests(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_video_frame_prompt_requests", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    options,
  });
}

export async function previewChapterVideoPromptRequests(
  projectId: Id,
  chapterId: Id,
  options: GeneratePromptOptions,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_video_prompt_requests", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    options,
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

export async function deleteChapterVideoPromptGroup(
  projectId: Id,
  chapterId: Id,
  group: "video_prompts" | "frame_prompts" | "all_chain_prompts",
  downstreamStrategy: DownstreamConflictStrategy = "reject",
): Promise<DeleteChapterVideoPromptGroupResult> {
  return apiInvoke<DeleteChapterVideoPromptGroupResult>("delete_chapter_video_prompt_group", {
    userId: currentUserId(),
    projectId: idNum(projectId, "项目 ID"),
    chapterId: idNum(chapterId, "章节 ID"),
    group,
    downstreamStrategy,
  });
}
