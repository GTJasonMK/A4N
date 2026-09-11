use crate::OrchestratorContext;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::RagSyncError;

pub(super) async fn current_embedding_identity(
    context: &OrchestratorContext,
) -> Result<EmbeddingModelIdentity, RagSyncError> {
    context
        .embedding_model_identity()
        .await
        .map_err(|error| RagSyncError::ReadEmbeddingIdentity {
            message: error.to_string(),
        })
}
