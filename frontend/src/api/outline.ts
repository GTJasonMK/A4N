import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";

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

/** 后端 generate_parts(input: { project_id, part_count?, chapters_per_part?, task_id?, variables? }) */
export async function generateParts(input: {
  project_id: number;
  part_count?: number | null;
  chapters_per_part?: number | null;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<PartOutline[]> {
  return apiInvoke<PartOutline[]>("generate_parts", { userId: currentUserId(), input });
}

export async function previewGeneratePartsRequest(input: {
  project_id: number;
  part_count?: number | null;
  chapters_per_part?: number | null;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_parts_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 continue_part_outlines(input: { project_id, part_number, start_chapter, end_chapter, variables? }) */
export async function continuePartOutlines(input: {
  project_id: number;
  part_number: number;
  start_chapter: number;
  end_chapter: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<PartOutline> {
  return apiInvoke<PartOutline>("continue_part_outlines", { userId: currentUserId(), input });
}

export async function previewContinuePartOutlinesRequest(input: {
  project_id: number;
  part_number: number;
  start_chapter: number;
  end_chapter: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_continue_part_outlines_request", {
    userId: currentUserId(),
    input,
  });
}

export async function regenerateParts(input: {
  project_id: number;
  part_count?: number | null;
  chapters_per_part?: number | null;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<PartOutline[]> {
  return apiInvoke<PartOutline[]>("regenerate_parts", { userId: currentUserId(), input });
}

export async function previewRegeneratePartsRequest(input: {
  project_id: number;
  part_count?: number | null;
  chapters_per_part?: number | null;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_parts_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 regenerate_last_part(input: { project_id, variables? }) */
export async function regenerateLastPart(input: {
  project_id: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<PartOutline> {
  return apiInvoke<PartOutline>("regenerate_last_part", { userId: currentUserId(), input });
}

export async function previewRegenerateLastPartRequest(input: {
  project_id: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_regenerate_last_part_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 regenerate_part_outline(input: { project_id, part_id, variables? }) */
export async function regeneratePartOutline(input: {
  project_id: number;
  part_id: number;
  task_id?: string | null;
  variables?: Record<string, unknown> | null;
}): Promise<PartOutline> {
  return apiInvoke<PartOutline>("regenerate_part_outline", { userId: currentUserId(), input });
}

export async function previewRegeneratePartOutlineRequest(input: {
  project_id: number;
  part_id: number;
  task_id?: string | null;
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

/** 后端 generate_part_chapter_outlines(input) */
export async function generatePartChapterOutlines(input: {
  project_id: number;
  part_id: number;
  task_id?: string | null;
}): Promise<ChapterOutline[]> {
  return apiInvoke<ChapterOutline[]>("generate_part_chapter_outlines", {
    userId: currentUserId(),
    input,
  });
}

export async function previewGeneratePartChapterOutlinesRequest(input: {
  project_id: number;
  part_id: number;
  task_id?: string | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_part_chapter_outlines_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 batch_generate_part_outlines(input) */
export async function batchGeneratePartOutlines(input: {
  project_id: number;
  part_ids: number[];
  task_id?: string | null;
}): Promise<ChapterOutline[]> {
  return apiInvoke<ChapterOutline[]>("batch_generate_part_outlines", {
    userId: currentUserId(),
    input,
  });
}

export async function previewBatchGeneratePartOutlinesRequest(input: {
  project_id: number;
  part_ids: number[];
  task_id?: string | null;
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

export async function applyBeatMapping(input: {
  project_id: number;
  part_id: number;
  beat_sheet: string;
  task_id?: string | null;
}): Promise<ChapterOutline[]> {
  return apiInvoke<ChapterOutline[]>("apply_beat_mapping", { userId: currentUserId(), input });
}

export async function previewApplyBeatMappingRequest(input: {
  project_id: number;
  part_id: number;
  beat_sheet: string;
  task_id?: string | null;
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

export async function generateChapterOutlinesByCount(
  input: {
    project_id: number;
    start_chapter: number;
    count: number;
    task_id?: string | null;
  },
): Promise<ChapterOutline[]> {
  return apiInvoke<ChapterOutline[]>("generate_chapter_outlines_by_count", {
    userId: currentUserId(),
    input,
  });
}

export async function previewGenerateChapterOutlinesByCountRequest(
  input: {
    project_id: number;
    start_chapter: number;
    count: number;
    task_id?: string | null;
  },
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_generate_chapter_outlines_by_count_request", {
    userId: currentUserId(),
    input,
  });
}

export async function regenerateChapterOutline(
  input: Record<string, unknown>,
): Promise<ChapterOutline> {
  return apiInvoke<ChapterOutline>("regenerate_chapter_outline", {
    userId: currentUserId(),
    input,
  });
}

export async function previewRegenerateChapterOutlineRequest(
  input: Record<string, unknown>,
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
