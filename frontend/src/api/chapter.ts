import { apiInvoke, extractErrorMessage } from "./index";
import { requireCurrentUserId as currentUserId } from "./authStorage";
import type {
  GenerationTaskHandle,
  GenerationTaskSummary,
  StartChapterGenerationInput,
} from "./generated/generationContracts";
import type {
  ChapterAnalysisInput,
  ChapterAnalysisMutationResult,
  ChapterSceneSummary,
  ChapterSummary,
  GenerateChapterSceneInput,
  ImportChapterInput,
  ReorderChapterScenesInput,
  UpdateChapterContentInput,
  UpdateChapterSceneInput,
} from "./generated/host";
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
 * Chapter API wrapper —— 对齐 a4n-api-contract::chapter 与通用 Host dispatch。
 *
 * 关键约定：
 *  - 后端 chapter id 是 i64 / chapter_number 是 i32（不是字符串）
 *  - 其它 input 类型命令统一 { input: {...} }
 */

export type {
  ChapterAnalysisInput,
  ChapterAnalysisMutationResult,
  ChapterMaterialSyncState,
  ChapterMaterialSyncStatus,
  ChapterMaterialUpdate,
  ChapterMaterialUpdateAction,
  ChapterMaterialUpdateKind,
  ChapterSceneSummary,
  ChapterSummary,
  GenerateChapterSceneInput,
  ImportChapterInput,
  ReorderChapterScenesInput,
  UpdateChapterContentInput,
  UpdateChapterSceneInput,
} from "./generated/host";
export {
  CHAPTER_ANALYSIS_PROGRESS_TOTAL,
  CHAPTER_SCENE_PROGRESS_TOTAL,
} from "./generated/host";

export const chapterContentCommitToastType = (
  derivedSyncStatus: string | null | undefined,
): "success" | "info" =>
  derivedSyncStatus === "failed" || derivedSyncStatus === "running" ? "info" : "success";

interface PreparedGenerationRagEmbeddingRequest {
  query_texts: string[];
  embedding_identity: {
    backend: string;
    model: string;
    dimension: number;
  };
  query_fingerprint: string;
}

type ClientGenerationRagQueryVectors = NonNullable<
  StartChapterGenerationInput["client_rag_query_vectors"]
>;

interface PreparedClientGenerationRagQuery {
  embeddingIdentity: PreparedGenerationRagEmbeddingRequest["embedding_identity"];
  queryFingerprint: string;
  queryVectors: ClientGenerationRagQueryVectors;
}

export async function listChapters(projectId: Id): Promise<ChapterSummary[]> {
  return apiInvoke<ChapterSummary[]>("list_chapters", {
    projectId: idNum(projectId),
  });
}

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

/** RAG 是可选增强，但合同/权限/身份错误不能被预取 fallback 静默掩盖。 */
export function shouldPropagateGenerationRagPrefetchError(error: unknown): boolean {
  const message = extractErrorMessage(error, "").toLowerCase();
  return (
    [
      "参数无效",
      "invalid argument",
      "identity",
      "身份",
      "权限不足",
      "未找到",
      "状态无效",
      "未认证",
    ].some((fragment) => message.includes(fragment))
    || /query vector|向量.*(?:不匹配|非有限|数量|维度)|embedding.*(?:mismatch|不匹配)/i.test(
      message,
    )
    || message.includes("返回数量不匹配")
  );
}

export async function startChapterGeneration(
  input: StartChapterGenerationInput,
): Promise<GenerationTaskHandle> {
  const projectId = idNum(input.project_id);
  let clientRagEmbeddingIdentity = input.client_rag_embedding_identity ?? null;
  let clientRagQueryFingerprint = input.client_rag_query_fingerprint ?? null;
  let clientRagQueryVectors = input.client_rag_query_vectors ?? null;
  if (
    canUseLocalDesktopEmbedding()
    && input.chapter_number > 1
    && clientRagEmbeddingIdentity === null
    && clientRagQueryFingerprint === null
    && clientRagQueryVectors === null
  ) {
    try {
      const preparedQuery = await prepareClientGenerationRagQuery(
        projectId,
        input.chapter_number,
        input.writing_notes,
        input.min_word_count,
        input.max_word_count,
      );
      clientRagEmbeddingIdentity = preparedQuery?.embeddingIdentity ?? null;
      clientRagQueryFingerprint = preparedQuery?.queryFingerprint ?? null;
      clientRagQueryVectors = preparedQuery?.queryVectors ?? null;
    } catch (error) {
      // RAG is an optional generation enhancement; apiInvoke has already logged the detail.
      if (shouldPropagateGenerationRagPrefetchError(error)) throw error;
      clientRagEmbeddingIdentity = null;
      clientRagQueryFingerprint = null;
      clientRagQueryVectors = null;
    }
  }
  return apiInvoke<GenerationTaskHandle>("start_chapter_generation", {
    userId: currentUserId(),
    input: {
      ...input,
      project_id: projectId,
      client_rag_embedding_identity: clientRagEmbeddingIdentity,
      client_rag_query_fingerprint: clientRagQueryFingerprint,
      client_rag_query_vectors: clientRagQueryVectors,
    },
  });
}

async function prepareClientGenerationRagQuery(
  projectId: number,
  chapterNumber: number,
  writingNotes?: string | null,
  minWordCount?: number | null,
  maxWordCount?: number | null,
): Promise<PreparedClientGenerationRagQuery | null> {
  const embeddingConfig = activeLocalEmbeddingConfig();
  // 本机没有默认 Embedding 时，RAG 只是可选增强；正文生成继续走服务端
  // 主链，避免在启动前把用户挡在“必须先配置资料检索”之外。
  if (!embeddingConfig) return null;
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
  const embedded = prepared.query_texts.length === 0
    ? { identity: prepared.embedding_identity, embeddings: [] }
    : await embedTextsLocally(embeddingConfig, prepared.query_texts);
  ensureEmbeddingIdentityMatches(embedded.identity, prepared.embedding_identity);
  ensureEmbeddingVectorCount(embedded.embeddings, prepared.query_texts.length);
  if (embedded.embeddings.length > 7) {
    throw new Error("章节生成 RAG 查询向量最多允许 7 组");
  }
  return {
    embeddingIdentity: embedded.identity,
    queryFingerprint: prepared.query_fingerprint,
    queryVectors: embedded.embeddings as ClientGenerationRagQueryVectors,
  };
}

// ============================================================================
// 导入/预览
// ============================================================================

export async function importChapter(input: ImportChapterInput): Promise<ChapterSummary> {
  return apiInvoke<ChapterSummary>("import_chapter", { input });
}

export async function updateChapterContent(
  input: UpdateChapterContentInput,
): Promise<ChapterSummary> {
  return apiInvoke<ChapterSummary>("update_chapter_content", { input });
}

export async function generateChapterAnalysis(
  input: ChapterAnalysisInput,
  taskId?: string | null,
): Promise<ChapterAnalysisMutationResult> {
  return apiInvoke<ChapterAnalysisMutationResult>("generate_chapter_analysis", {
    input,
    taskId: taskId ?? null,
  });
}

export async function previewChapterAnalysisRequest(
  input: ChapterAnalysisInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_analysis_request", {
    input,
  });
}

export async function resumeChapterAnalysisSync(
  input: ChapterAnalysisInput,
  taskId?: string | null,
): Promise<ChapterSummary> {
  return apiInvoke<ChapterSummary>("resume_chapter_analysis_sync", {
    input,
    taskId: taskId ?? null,
  });
}

export async function previewResumeChapterAnalysisSyncRequest(
  input: ChapterAnalysisInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_resume_chapter_analysis_sync_request", {
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

export async function listChapterScenes(chapterId: Id): Promise<ChapterSceneSummary[]> {
  return apiInvoke<ChapterSceneSummary[]>("list_chapter_scenes", {
    chapterId: idNum(chapterId),
  });
}

export async function generateChapterScene(
  input: GenerateChapterSceneInput,
  taskId?: string | null,
): Promise<ChapterSceneSummary> {
  return apiInvoke<ChapterSceneSummary>("generate_chapter_scene", {
    input,
    taskId: taskId ?? null,
  });
}

export async function previewChapterSceneRequest(
  input: GenerateChapterSceneInput,
): Promise<ModelRequestPreview> {
  return apiInvoke<ModelRequestPreview>("preview_chapter_scene_request", {
    input,
  });
}

export async function updateChapterScene(
  input: UpdateChapterSceneInput,
): Promise<ChapterSceneSummary> {
  return apiInvoke<ChapterSceneSummary>("update_chapter_scene", { input });
}

export async function reorderChapterScenes(
  input: ReorderChapterScenesInput,
): Promise<ChapterSceneSummary[]> {
  return apiInvoke<ChapterSceneSummary[]>("reorder_chapter_scenes", { input });
}

export async function deleteChapterScene(sceneId: Id): Promise<void> {
  return apiInvoke<void>("delete_chapter_scene", {
    sceneId: idNum(sceneId),
  });
}
