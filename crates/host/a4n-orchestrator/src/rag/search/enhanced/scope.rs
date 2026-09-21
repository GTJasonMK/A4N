use a4n_embedding_provider_contract::EmbeddingModelIdentity;
use a4n_error::AppError;
use a4n_rag_core::{
    search::identity_scoped_search_filter, sync::ensure_project_embedding_identity_matches,
    vector_store::SearchFilter,
};

use crate::OrchestratorContext;

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
        ensure_project_embedding_identity_matches(&stored_metadata, embedding_identity)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        Ok(identity_scoped_search_filter(
            embedding_identity.clone(),
            data_types,
        ))
    }
}
