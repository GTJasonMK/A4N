use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;

use super::contracts::RagEmbeddedInput;
use crate::OrchestratorContext;

impl OrchestratorContext {
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
