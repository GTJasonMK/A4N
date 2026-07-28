use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;

use super::contracts::{RagEmbeddedInput, RagEmbeddingInput};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn embed_and_store_rag_record(
        &self,
        project_id: i64,
        chunk_id: &str,
        text: &str,
        metadata: serde_json::Value,
    ) -> Result<Vec<f32>, AppError> {
        self.rag_embed_and_store_record(project_id, chunk_id, text, metadata)
            .await
    }

    pub async fn embed_and_store_rag_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddingInput>,
    ) -> Result<usize, AppError> {
        self.rag_embed_and_store_records(project_id, inputs).await
    }

    pub async fn store_embedded_rag_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<usize, AppError> {
        self.rag_store_embedded_records(project_id, inputs, embedding_identity)
            .await
    }
}
