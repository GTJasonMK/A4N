use std::path::Path;

use a4n_core::error::AppError;
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::{FastEmbedLocalModel, list_fastembed_local_models};

use crate::AppState;

use super::super::{
    contracts::{EmbeddingModelExecution, LocalEmbeddingModelSummary},
    provider::fastembed_cache_dir,
    rules::embedding_model_execution,
};

pub async fn list_local_embedding_models(
    state: &AppState,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    if embedding_model_execution(state.runtime_profile) == EmbeddingModelExecution::Sidecar {
        return crate::sidecar_clients::embedding_model_operation(state.capabilities(), "list", "")
            .await;
    }
    list_local_embedding_models_in_cache(&fastembed_cache_dir(state))
}

pub fn list_local_embedding_models_in_cache(
    cache_dir: &Path,
) -> Result<Vec<LocalEmbeddingModelSummary>, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        Ok(list_fastembed_local_models(cache_dir)?
            .into_iter()
            .map(to_local_embedding_model_summary)
            .collect())
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = cache_dir;
        Err(super::super::provider::local_embedding_disabled_error())
    }
}

#[cfg(feature = "local-embedding")]
fn to_local_embedding_model_summary(model: FastEmbedLocalModel) -> LocalEmbeddingModelSummary {
    LocalEmbeddingModelSummary {
        name: model.name,
        model_code: model.model_code,
        description: model.description,
        dimension: model.dimension,
        downloaded: model.downloaded,
        cache_path: model.cache_path.to_string_lossy().to_string(),
        size_bytes: model.size_bytes,
    }
}
