import { apiInvoke } from "../index";
import { activeLocalEmbeddingConfig } from "../localRuntimeSettings";
import { embedTextsLocally } from "../config/embedding";
import {
  ensureEmbeddingIdentityMatches,
  ensureEmbeddingVectorCount,
} from "../embeddingIdentity";
import { canUseLocalDesktopEmbedding } from "../../utils/runtimeMode";
import type {
  EnhancedSearchInput,
  RagEnhancedSearchOutput,
  RagPreparedEnhancedSearchOutput,
} from "./types";
import { type ApiId as Id, idNum } from "../shared";

export async function ragEnhancedSearch(
  projectId: Id,
  input: EnhancedSearchInput,
): Promise<RagEnhancedSearchOutput> {
  const normalizedInput = {
    query: input.query,
    top_k: input.top_k,
    chapter_number: input.chapter_number ?? null,
    data_types: input.data_types ?? null,
  };
  if (canUseLocalDesktopEmbedding()) {
    const embeddingConfig = activeLocalEmbeddingConfig();
    const prepared = await apiInvoke<RagPreparedEnhancedSearchOutput>(
      "rag_prepare_enhanced_search",
      {
        projectId: idNum(projectId),
        input: normalizedInput,
      },
    );
    const embedded = await embedTextsLocally(embeddingConfig, prepared.query_texts);
    ensureEmbeddingIdentityMatches(embedded.identity, prepared.embedding_identity);
    ensureEmbeddingVectorCount(embedded.embeddings, prepared.query_texts.length);
    return apiInvoke<RagEnhancedSearchOutput>("rag_enhanced_search_with_vectors", {
      projectId: idNum(projectId),
      input: {
        search: normalizedInput,
        embedding_identity: prepared.embedding_identity,
        query_vectors: embedded.embeddings,
      },
    });
  }
  return apiInvoke<RagEnhancedSearchOutput>("rag_enhanced_search", {
    projectId: idNum(projectId),
    input: normalizedInput,
  });
}
