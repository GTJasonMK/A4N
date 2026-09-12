use std::path::Path;

#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::fastembed_model_download_info;
use a4n_error::AppError;

use crate::AppState;

use super::super::{
    EmbeddingModelDownloadInfoSummary, EmbeddingModelInput, host_adapter::required_text,
};

pub async fn get_embedding_model_download_info(
    state: &AppState,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    if state.runtime_profile().uses_sidecars() {
        let model = required_text(&input.model, "fastembed 模型名")?;
        return crate::sidecar_clients::embedding_model_download_info(
            state.capabilities().embedding.clone(),
            &model,
        )
        .await;
    }
    get_embedding_model_download_info_in_cache(&input.model, &state.embedding_models_dir()).await
}

pub async fn get_embedding_model_download_info_in_cache(
    model: &str,
    cache_dir: &Path,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let model = required_text(model, "fastembed 模型名")?;
        fastembed_model_download_info(&model, cache_dir.to_path_buf()).await
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (model, cache_dir);
        Err(super::super::provider::local_embedding_disabled_error())
    }
}
