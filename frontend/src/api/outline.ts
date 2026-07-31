import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import { generationClient } from "./generationTasks";
import type {
  GenerationTaskHandle,
  StartOutlineGenerationInput,
} from "./generated/generationContracts";

/**
 * Outline API wrapper —— 对齐 crates/a4n-tauri/src/commands/outline.rs
 *
 * 注意：A4N 后端的章节大纲（chapter_outline）按 **project_id** 索引，不是 part_id。
 * 前端之前用 partId 是错的。
 */

export interface PartOutline {
  id: number;
  project_id: number;
  part_number: number;
  start_chapter: number;
  end_chapter: number;
  title: string | null;
  summary: string | null;
}

export interface ChapterOutline {
  id: number;
  project_id: number;
  chapter_number: number;
  title: string | null;
  summary: string | null;
  word_count_target?: number | null;
}

export interface GenerationProgress {
  total_parts: number;
  completed_parts: number;
  total_chapter_outlines: number;
  completed_chapter_outlines: number;
}

export interface ChapterOutlinePacing {
  pacing_score: number;
  chapter_count: number;
  summary?: string;
  analysis: Array<{
    chapter_number: number;
    title: string | null;
    pacing_score: number;
    word_count_target: number | null;
    has_beats: boolean;
    has_key_scenes: boolean;
  }>;
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问大纲");
  return id;
};

// ============================================================================
// 分部大纲
// ============================================================================

export async function listParts(projectId: Id): Promise<PartOutline[]> {
  return apiInvoke<PartOutline[]>("list_parts", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export type OutlineGenerationInput = Omit<StartOutlineGenerationInput, "meta">;

export async function startOutlineGeneration(
  input: OutlineGenerationInput,
): Promise<GenerationTaskHandle> {
  return generationClient.start((meta) =>
    apiInvoke<GenerationTaskHandle>("start_outline_generation", {
      userId: currentUserId(),
      input: { ...input, meta, project_id: idNum(input.project_id) },
    }),
  );
}

export async function previewGeneratePartsRequest(input: {
  project_id: number;
  part_count?: number | null;
  chapters_per_part?: number | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_parts_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewContinuePartOutlinesRequest(input: {
  project_id: number;
  part_number: number;
  start_chapter: number;
  end_chapter: number;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_continue_part_outlines_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewRegeneratePartsRequest(input: {
  project_id: number;
  part_count?: number | null;
  chapters_per_part?: number | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_parts_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewRegenerateLastPartRequest(input: {
  project_id: number;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_last_part_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewRegeneratePartOutlineRequest(input: {
  project_id: number;
  part_id: number;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_part_outline_request", {
    userId: currentUserId(),
    input,
  });
}

export async function deleteLatestParts(projectId: Id, count: number): Promise<void> {
  return apiInvoke<void>("delete_latest_parts", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    count,
  });
}

export async function previewGeneratePartChapterOutlinesRequest(input: {
  project_id: number;
  part_id: number;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_part_chapter_outlines_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewBatchGeneratePartOutlinesRequest(input: {
  project_id: number;
  part_ids: number[];
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_batch_generate_part_outlines_request", {
    userId: currentUserId(),
    input,
  });
}

export async function getGenerationProgress(projectId: Id): Promise<GenerationProgress> {
  return apiInvoke<GenerationProgress>("get_generation_progress", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function previewApplyBeatMappingRequest(input: {
  project_id: number;
  part_id: number;
  beat_sheet: string;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_apply_beat_mapping_request", {
    userId: currentUserId(),
    input,
  });
}

// ============================================================================
// 章节大纲（按 project_id 索引）
// ============================================================================

export async function listChapterOutlines(projectId: Id): Promise<ChapterOutline[]> {
  return apiInvoke<ChapterOutline[]>("list_chapter_outlines", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function previewGenerateChapterOutlinesByCountRequest(
  input: {
    project_id: number;
    start_chapter: number;
    count: number;
  },
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_chapter_outlines_by_count_request", {
    userId: currentUserId(),
    input,
  });
}

export async function previewRegenerateChapterOutlineRequest(
  input: {
    project_id: number;
    outline_id: number;
    variables?: Record<string, unknown> | null;
  },
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_chapter_outline_request", {
    userId: currentUserId(),
    input,
  });
}

export async function updateChapterOutline(input: {
  project_id: number;
  outline_id: number;
  title?: string;
  summary?: string;
  beats?: unknown[];
  key_scenes?: unknown[];
}): Promise<ChapterOutline> {
  return apiInvoke<ChapterOutline>("update_chapter_outline", { userId: currentUserId(), input });
}

export async function deleteLatestChapterOutlines(
  projectId: Id,
  count: number,
): Promise<void> {
  return apiInvoke<void>("delete_latest_chapter_outlines", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    count,
  });
}

export async function getChapterOutlinesPacing(projectId: Id): Promise<ChapterOutlinePacing> {
  return apiInvoke<ChapterOutlinePacing>("get_chapter_outlines_pacing", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}
