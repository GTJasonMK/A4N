import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type {
  GenerationTaskHandle,
  GenerationTaskSummary,
  StartChapterGenerationInput,
} from "./generated/generationContracts";
import type { ModelRequestPreview } from "./requestPreview";
import { idNum, type ApiId as Id } from "./shared";
import { activeLocalEmbeddingConfig } from "./localRuntimeSettings";
import { embedTextsLocally } from "./config/embedding";
import {
  ensureEmbeddingIdentityMatches,
  ensureEmbeddingVectorCount,
} from "./embeddingIdentity";
import { canUseLocalDesktopEmbedding } from "../utils/runtimeMode";

/**
 * Chapter API wrapper —— 对齐 crates/a4n-tauri/src/commands/chapter.rs
 *
 * 关键约定：
 *  - 后端 chapter id 是 i64 / chapter_number 是 i32（不是字符串）
 *  - 其它 input 类型命令统一 { input: {...} }
 */

// ============================================================================
// 类型（对齐后端 *Summary）
// ============================================================================

export interface ChapterData {
  id: number;
  project_id: number;
  outline_id: number | null;
  chapter_number: number;
  title: string | null;
  content: string;
  word_count: number;
  status: string;
  summary: string | null;
  style_tags: unknown | null;
  analysis_data: unknown | null;
  analysis_sync_status: string | null;
  analysis_sync_can_resume: boolean;
  analysis_sync_resume_block_reason: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface ChapterScene {
  id: number;
  chapter_id: number;
  scene_number: number;
  title: string | null;
  content: string;
  word_count: number;
  summary: string | null;
  scene_type: string;
  characters_present: unknown | null;
  location: string | null;
  timeline_position: string | null;
  transition_from_previous: string | null;
  created_at: string | null;
  updated_at: string | null;
}

interface PreparedGenerationRagEmbeddingRequest {
  query_texts: string[];
  embedding_identity: {
    backend: string;
    model: string;
    dimension: number;
  };
}

interface GenerationRagContext {
  text?: string | null;
}

export async function listChapters(projectId: Id): Promise<ChapterData[]> {
  return apiInvoke<ChapterData[]>("list_chapters", {
    userId: currentUserId(),
    projectId: idNum(projectId),
  });
}

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问章节");
  return id;
};

// ============================================================================
// 章节生成
// ============================================================================

export function chapterGenerationNumber(
  summary: GenerationTaskSummary,
  projectId: number,
): number | null {
  const target = summary.started.target;
  const match = target?.kind === "chapter"
    ? /^([1-9]\d*):([1-9]\d*)$/.exec(target.id)
    : null;
  if (!match || summary.started.project_id !== projectId) return null;
  const targetProjectId = Number(match[1]);
  const chapterNumber = Number(match[2]);
  return Number.isSafeInteger(targetProjectId)
    && Number.isSafeInteger(chapterNumber)
    && targetProjectId === projectId
    ? chapterNumber
    : null;
}

export async function startChapterGeneration(
  input: StartChapterGenerationInput,
): Promise<GenerationTaskHandle> {
  const projectId = idNum(input.project_id);
  let clientRagContext = input.client_rag_context ?? null;
  if (
    canUseLocalDesktopEmbedding()
    && input.chapter_number > 1
    && !clientRagContext?.trim()
  ) {
    clientRagContext = await prepareClientGenerationRagContext(
      projectId,
      input.chapter_number,
      input.writing_notes,
      input.min_word_count,
      input.max_word_count,
    );
  }
  return apiInvoke<GenerationTaskHandle>("start_chapter_generation", {
    userId: currentUserId(),
    input: {
      ...input,
      project_id: projectId,
      client_rag_context: clientRagContext,
    },
  });
}

async function prepareClientGenerationRagContext(
  projectId: number,
  chapterNumber: number,
  writingNotes?: string | null,
  minWordCount?: number | null,
  maxWordCount?: number | null,
): Promise<string | null> {
  const embeddingConfig = activeLocalEmbeddingConfig();
  if (!embeddingConfig) throw new Error("请先设置默认 Embedding 配置");
  const prepared = await apiInvoke<PreparedGenerationRagEmbeddingRequest>(
    "prepare_chapter_generation_rag_embedding",
    {
      userId: currentUserId(),
      projectId,
      chapterNumber,
      writingNotes: writingNotes ?? null,
      minWordCount: minWordCount ?? null,
      maxWordCount: maxWordCount ?? null,
    },
  );
  if (prepared.query_texts.length === 0) return null;
  const embedded = await embedTextsLocally(embeddingConfig, prepared.query_texts);
  ensureEmbeddingIdentityMatches(embedded.identity, prepared.embedding_identity);
  ensureEmbeddingVectorCount(embedded.embeddings, prepared.query_texts.length);
  const context = await apiInvoke<GenerationRagContext>(
    "finish_chapter_generation_rag_context",
    {
      userId: currentUserId(),
      input: {
        project_id: projectId,
        chapter_number: chapterNumber,
        writing_notes: writingNotes ?? null,
        embedding_identity: embedded.identity,
        query_vectors: embedded.embeddings,
      },
    },
  );
  return context.text ?? null;
}

// ============================================================================
// 导入/预览
// ============================================================================

/** 后端 import_chapter(input: { project_id, chapter_number, content, title? }) */
export async function importChapter(input: {
  project_id: number;
  chapter_number: number;
  content: string;
  title?: string | null;
}): Promise<unknown> {
  return apiInvoke<unknown>("import_chapter", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 update_chapter_content(input: { chapter_id, content, title? }) */
export async function updateChapterContent(input: {
  chapter_id: number;
  content: string;
  title?: string | null;
}): Promise<ChapterData> {
  return apiInvoke<ChapterData>("update_chapter_content", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 generate_chapter_analysis(input: { chapter_id }) */
export async function generateChapterAnalysis(input: {
  chapter_id: number;
}, taskId?: string | null): Promise<ChapterData> {
  return apiInvoke<ChapterData>("generate_chapter_analysis", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewChapterAnalysisRequest(input: {
  chapter_id: number;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_analysis_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 resume_chapter_analysis_sync(input: { chapter_id }) —— 复用已保存分析继续同步派生数据 */
export async function resumeChapterAnalysisSync(input: {
  chapter_id: number;
}, taskId?: string | null): Promise<ChapterData> {
  return apiInvoke<ChapterData>("resume_chapter_analysis_sync", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewResumeChapterAnalysisSyncRequest(input: {
  chapter_id: number;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_resume_chapter_analysis_sync_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 preview_chapter_prompt(input: { project_id, chapter_number, writing_notes?, min_word_count?, max_word_count? }) */
export async function previewChapterPrompt(input: {
  project_id: number;
  chapter_number: number;
  writing_notes?: string | null;
  min_word_count?: number | null;
  max_word_count?: number | null;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_prompt", {
    userId: currentUserId(),
    input,
  });
}

// ============================================================================
// 场景管理
// ============================================================================

export async function listChapterScenes(chapterId: Id): Promise<ChapterScene[]> {
  return apiInvoke<ChapterScene[]>("list_chapter_scenes", {
    userId: currentUserId(),
    chapterId: idNum(chapterId),
  });
}

/** 后端 generate_chapter_scene(input: { chapter_id, scene_number, scene_prompt }) */
export async function generateChapterScene(input: {
  chapter_id: number;
  scene_number: number;
  scene_prompt: string;
}, taskId?: string | null): Promise<ChapterScene> {
  return apiInvoke<ChapterScene>("generate_chapter_scene", {
    userId: currentUserId(),
    input,
    taskId: taskId ?? null,
  });
}

export async function previewChapterSceneRequest(input: {
  chapter_id: number;
  scene_number: number;
  scene_prompt: string;
}): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_scene_request", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 update_chapter_scene(input: { scene_id, content?, title?, summary?, scene_type?, location?, timeline_position? }) */
export async function updateChapterScene(input: {
  scene_id: number;
  content?: string | null;
  title?: string | null;
  summary?: string | null;
  scene_type?: string | null;
  location?: string | null;
  timeline_position?: string | null;
}): Promise<ChapterScene> {
  return apiInvoke<ChapterScene>("update_chapter_scene", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 reorder_chapter_scenes(input: { order: [[scene_id, scene_number], ...] }) */
export async function reorderChapterScenes(input: {
  order: Array<[number, number]>;
}): Promise<ChapterScene[]> {
  return apiInvoke<ChapterScene[]>("reorder_chapter_scenes", {
    userId: currentUserId(),
    input,
  });
}

/** 后端 delete_chapter_scene(scene_id) */
export async function deleteChapterScene(sceneId: Id): Promise<void> {
  return apiInvoke<void>("delete_chapter_scene", {
    userId: currentUserId(),
    sceneId: idNum(sceneId),
  });
}
