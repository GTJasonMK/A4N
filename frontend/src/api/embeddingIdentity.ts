import type { EmbeddingModelIdentity } from "./config/types";

const embeddingIdentityLabel = (identity: EmbeddingModelIdentity): string =>
  `${identity.backend}/${identity.model}/${identity.dimension}`;

export const ensureEmbeddingIdentityMatches = (
  actual: EmbeddingModelIdentity,
  expected: EmbeddingModelIdentity,
): void => {
  if (
    actual.backend === expected.backend &&
    actual.model === expected.model &&
    actual.dimension === expected.dimension
  ) {
    return;
  }
  throw new Error(
    `本机 Embedding identity 不匹配：服务器期望 ${embeddingIdentityLabel(expected)}，本机返回 ${embeddingIdentityLabel(actual)}`,
  );
};

export const ensureEmbeddingVectorCount = (
  vectors: number[][],
  expectedCount: number,
): void => {
  if (vectors.length === expectedCount) return;
  throw new Error(
    `本机 Embedding 返回数量不匹配：expected=${expectedCount}, actual=${vectors.length}`,
  );
};
