import { apiInvoke } from "../index";
import { requireCurrentUserId as currentUserId } from "../authStorage";
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

export const formatRagIngestResultSummary = (result: RagIngestResult): string =>
  `写入 ${result.chunk_count}，新增 ${result.added_count}，更新 ${result.updated_count}，跳过 ${result.skipped_count}，删除 ${result.deleted_count}`;

export async function ragIngestAll(
  projectId: Id,
  taskId?: string | null,
  rebuild = false,
): Promise<RagIngestResult> {
  if (canUseLocalDesktopEmbedding()) {
    return ragIngestAllWithLocalDesktopEmbedding(projectId, rebuild);
  }
  return apiInvoke<RagIngestResult>("rag_ingest_all", {
    userId: currentUserId(),
    projectId: idNum(projectId),
    rebuild,
    taskId: taskId ?? null,
  });
}

async function ragIngestAllWithLocalDesktopEmbedding(
  projectId: Id,
  rebuild: boolean,
): Promise<RagIngestResult> {
  const projectIdNum = idNum(projectId);
  const embeddingConfig = activeLocalEmbeddingConfig();

  const prepared = await apiInvoke<ClientRagIngestPrepareOutput>("rag_prepare_ingest_all", {
    userId: currentUserId(),
    projectId: projectIdNum,
    rebuild,
  });

  const uploaded: Array<{
    chunk_id: string;
    content_hash: string;
    embedding: number[];
  }> = [];
  const batchSize = embeddingConfig.batch_size;
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
        content_hash: batch[index].content_hash,
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
      rebuild: prepared.rebuild,
      embeddings: uploaded,
    },
  });
}
