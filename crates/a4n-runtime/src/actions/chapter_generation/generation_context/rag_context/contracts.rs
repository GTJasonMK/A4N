use a4n_embedding::provider::EmbeddingModelIdentity;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PreparedGenerationRagEmbeddingRequest {
    pub query_texts: Vec<String>,
    pub embedding_identity: EmbeddingModelIdentity,
}
