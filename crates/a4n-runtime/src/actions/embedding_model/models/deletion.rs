use std::path::Path;

#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::delete_fastembed_model;
use a4n_error::AppError;

use crate::AppState;

use super::super::{EmbeddingModelInput, host_adapter::required_text};

pub async fn delete_local_embedding_model(
    state: &AppState,
    input: EmbeddingModelInput,
) -> Result<(), AppError> {
    if state.runtime_profile().uses_sidecars() {
        let model = required_text(&input.model, "fastembed 模型名")?;
        return crate::sidecar_clients::delete_embedding_model(
            state.capabilities().embedding.clone(),
            &model,
        )
        .await;
    }
    delete_local_embedding_model_in_cache(&input.model, &state.embedding_models_dir())
}

pub fn delete_local_embedding_model_in_cache(
    model: &str,
    cache_dir: &Path,
) -> Result<(), AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let model = required_text(model, "fastembed 模型名")?;
        delete_fastembed_model(&model, cache_dir)
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (model, cache_dir);
        Err(super::super::provider::local_embedding_disabled_error())
    }
}
