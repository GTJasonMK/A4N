use std::path::Path;

#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::list_fastembed_local_models;
use a4n_error::AppError;

use crate::AppState;

use super::super::LocalEmbeddingModelSummary;

pub async fn list_local_embedding_models(
    state: &AppState,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    if state.runtime_profile().uses_sidecars() {
        return crate::sidecar_clients::list_embedding_models(
            state.capabilities().embedding.clone(),
        )
        .await;
    }
    list_local_embedding_models_in_cache(&state.embedding_models_dir())
}

pub fn list_local_embedding_models_in_cache(
    cache_dir: &Path,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        list_fastembed_local_models(cache_dir)
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = cache_dir;
        Err(super::super::provider::local_embedding_disabled_error())
    }
}
