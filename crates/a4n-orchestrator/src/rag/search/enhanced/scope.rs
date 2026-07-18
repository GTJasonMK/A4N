use a4n_core::error::AppError;
use a4n_embedding::provider::EmbeddingModelIdentity;
use a4n_rag_core::vector_store::SearchFilter;

use crate::{OrchestratorContext, rag};

impl OrchestratorContext {
    pub async fn enhanced_rag_search_filter(
        &self,
        project_id: i64,
        embedding_identity: &EmbeddingModelIdentity,
        data_types: Option<Vec<String>>,
    ) -> Result<SearchFilter, AppError> {
        let stored_metadata = self
            .project_rag_metadata(project_id)
            .await
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        rag::ensure_project_embedding_identity_matches(&stored_metadata, embedding_identity)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        Ok(rag::identity_scoped_search_filter(
            project_id,
            embedding_identity.clone(),
            data_types,
        ))
    }
}
