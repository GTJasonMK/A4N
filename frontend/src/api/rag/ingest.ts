import { apiInvoke } from "../index";
import { readCurrentUserId } from "../authStorage";
import { activeLocalEmbeddingConfig } from "../localRuntimeSettings";
import { embedTextsLocally } from "../config/embedding";
import {
  ensureEmbeddingIdentityMatches,
  ensureEmbeddingVectorCount,
} from "../embeddingIdentity";
import { canUseLocalDesktopEmbedding } from "../../utils/runtimeMode";
import type {
  ClientRagIngestPrepareOutput,
  RagIngestResult,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法同步 RAG");
  return id;
};

export const formatRagIngestResultSummary = (result: RagIngestResult): string =>
  `写入 ${result.chunk_count}，新增 ${result.added_count}，更新 ${result.updated_count}，跳过 ${result.skipped_count}，删除 ${result.deleted_count}`;

export async function ragIngestAll(
  projectId: Id,
  taskId?: string | null,
): Promise<RagIngestResult> {
  if (canUseLocalDesktopEmbedding()) {
    return ragIngestAllWithLocalDesktopEmbedding(projectId);
  }
  return apiInvoke<RagIngestResult>("rag_ingest_all", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    taskId: taskId ?? null,
  });
}

async function ragIngestAllWithLocalDesktopEmbedding(projectId: Id): Promise<RagIngestResult> {
  const projectIdNum = idNum(projectId);
  const embeddingConfig = activeLocalEmbeddingConfig();
  if (!embeddingConfig) throw new Error("请先设置默认 Embedding 配置");

  const prepared = await apiInvoke<ClientRagIngestPrepareOutput>("rag_prepare_ingest_all", {
    userId: currentUserId(),
    projectId: projectIdNum,
  });

  const uploaded: Array<{ chunk_id: string; embedding: number[] }> = [];
  const batchSize = Math.max(1, embeddingConfig.batch_size || 32);
  for (let start = 0; start < prepared.records.length; start += batchSize) {
    const batch = prepared.records.slice(start, start + batchSize);
    const embedded = await embedTextsLocally(
      embeddingConfig,
      batch.map((record) => record.text),
    );
    ensureEmbeddingIdentityMatches(embedded.identity, prepared.embedding_identity);
    ensureEmbeddingVectorCount(embedded.embeddings, batch.length);
    for (let index = 0; index < batch.length; index += 1) {
      uploaded.push({
        chunk_id: batch[index].chunk_id,
        embedding: embedded.embeddings[index],
      });
    }
  }

  return apiInvoke<RagIngestResult>("rag_upload_ingest_embeddings", {
    userId: currentUserId(),
    projectId: projectIdNum,
    input: {
      operation_trace_id: prepared.operation_trace_id,
      embedding_identity: prepared.embedding_identity,
      embeddings: uploaded,
    },
  });
}
