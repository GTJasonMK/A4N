import { apiInvoke, extractErrorMessage } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import { generationClient } from "./generationTasks";
import type {
  GenerationTaskHandle,
  StartOutlineGenerationInput,
} from "./generated/generationContracts";
import type {
  ChapterOutlineSummary,
  DeleteLatestOutlineRecordsInput,
  OutlineDeleteTarget,
  OutlineMutationResult,
  PartOutlineSummary,
  SaveChapterOutlineInput,
  SaveChapterOutlineResult,
  SavePartOutlineInput,
  SavePartOutlineResult,
  UpdateChapterOutlineResult,
} from "./generated/host";

/**
 * Outline API wrapper —— 对齐 crates/a4n-tauri/src/commands/outline.rs
 *
 * 注意：A4N 后端的章节大纲（chapter_outline）按 **project_id** 索引，不是 part_id。
 * 前端之前用 partId 是错的。
 */

export type PartOutline = PartOutlineSummary;

export type ChapterOutline = ChapterOutlineSummary;

export const CHAPTER_OUTLINE_SOURCE_RETROSPECTIVE_ANALYSIS = "retrospective_analysis";

export const isDerivedChapterOutline = (outline: ChapterOutline | null | undefined): boolean =>
  Boolean(outline?.source_kind?.trim());

export const isRetrospectiveChapterOutline = (
  outline: ChapterOutline | null | undefined,
): boolean => outline?.source_kind === CHAPTER_OUTLINE_SOURCE_RETROSPECTIVE_ANALYSIS;

export const chapterOutlineSourceLabel = (
  outline: ChapterOutline | null | undefined,
): string => {
  if (isRetrospectiveChapterOutline(outline)) return "回顾摘要";
  if (isDerivedChapterOutline(outline)) return "分析候选";
  return "章节大纲";
};

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

// ============================================================================
// 分部大纲
// ============================================================================

export async function listParts(projectId: Id): Promise<PartOutline[]> {
  return apiInvoke<PartOutline[]>("list_parts", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

export async function savePartOutline(
  input: SavePartOutlineInput,
): Promise<SavePartOutlineResult> {
  return apiInvoke<SavePartOutlineResult>("save_part_outline", {
    userId: currentUserId(),
    input,
  });
}

export async function syncOutlineMaterials(projectId: Id): Promise<string[]> {
  const normalizedProjectId = idNum(projectId);
  try {
    const result = await apiInvoke<OutlineMutationResult>("sync_outline_materials", {
      projectId: normalizedProjectId,
    });
    return result.warnings;
  } catch (error) {
    return [
      `资料检索索引更新失败：${extractErrorMessage(error, "同步命令调用失败")}。可在项目操作中重新同步资料索引`,
    ];
  }
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
  expected_parts: OutlineDeleteTarget[];
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

export async function deleteLatestParts(
  projectId: Id,
  parts: ReadonlyArray<Pick<PartOutline, "id" | "updated_at">>,
): Promise<OutlineMutationResult> {
  return apiInvoke<OutlineMutationResult>("delete_latest_parts", {
    userId: currentUserId(),
    input: outlineDeleteInput(projectId, parts),
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

export async function saveChapterOutline(
  input: SaveChapterOutlineInput,
): Promise<SaveChapterOutlineResult> {
  return apiInvoke<SaveChapterOutlineResult>("save_chapter_outline", {
    userId: currentUserId(),
    input,
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
  expected_updated_at: string;
  title?: string;
  summary?: string;
  beats?: unknown[];
  key_scenes?: unknown[];
}): Promise<UpdateChapterOutlineResult> {
  return apiInvoke<UpdateChapterOutlineResult>("update_chapter_outline", {
    userId: currentUserId(),
    input,
  });
}

export async function deleteLatestChapterOutlines(
  projectId: Id,
  outlines: ReadonlyArray<Pick<ChapterOutline, "id" | "updated_at">>,
): Promise<OutlineMutationResult> {
  return apiInvoke<OutlineMutationResult>("delete_latest_chapter_outlines", {
    userId: currentUserId(),
    input: outlineDeleteInput(projectId, outlines),
  });
}

function outlineDeleteInput(
  projectId: Id,
  records: ReadonlyArray<{ id: number; updated_at?: string | null }>,
): DeleteLatestOutlineRecordsInput {
  return {
    project_id: idNum(projectId),
    targets: outlineRecordTargets(records),
  };
}

export function outlineRecordTargets(
  records: ReadonlyArray<{ id: number; updated_at?: string | null }>,
): [OutlineDeleteTarget, ...OutlineDeleteTarget[]] {
  if (records.length === 0) throw new Error("至少选择一条大纲资料");
  const targets = records.map((record) => {
    const expectedUpdatedAt = record.updated_at?.trim();
    if (!expectedUpdatedAt) throw new Error("缺少大纲资料版本，请刷新后重试");
    return { id: record.id, expected_updated_at: expectedUpdatedAt };
  });
  return [targets[0], ...targets.slice(1)];
}

export async function getChapterOutlinesPacing(projectId: Id): Promise<ChapterOutlinePacing> {
  return apiInvoke<ChapterOutlinePacing>("get_chapter_outlines_pacing", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}
