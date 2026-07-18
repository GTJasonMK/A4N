use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;

use super::contracts::{RagEmbeddedInput, RagEmbeddingInput};
use super::embedded::store_embedded_rag_records_in_store;
use super::provider::{embed_and_store_rag_record_in_store, embed_and_store_rag_records_in_store};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn embed_and_store_rag_record(
        &self,
        project_id: i64,
        chunk_id: &str,
        text: &str,
        metadata: serde_json::Value,
    ) -> Result<Vec<f32>, AppError> {
        embed_and_store_rag_record_in_store(
            self.rag_store.as_ref(),
            project_id,
            chunk_id,
            text,
            self.embedding.as_ref(),
            metadata,
        )
        .await
    }

    pub async fn embed_and_store_rag_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddingInput>,
    ) -> Result<usize, AppError> {
        embed_and_store_rag_records_in_store(
            self.rag_store.as_ref(),
            project_id,
            inputs,
            self.embedding.as_ref(),
        )
        .await
    }

    pub async fn store_embedded_rag_records(
        &self,
        project_id: i64,
        inputs: Vec<RagEmbeddedInput>,
        embedding_identity: EmbeddingModelIdentity,
    ) -> Result<usize, AppError> {
        store_embedded_rag_records_in_store(
            self.rag_store.as_ref(),
            project_id,
            inputs,
            embedding_identity,
        )
        .await
    }
}
