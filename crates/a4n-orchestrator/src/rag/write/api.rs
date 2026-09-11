use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_error::AppError;

use crate::OrchestratorContext;
use a4n_rag_core::RagEmbeddedInput;

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
