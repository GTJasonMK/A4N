use std::path::Path;

use a4n_core::error::AppError;
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::{FastEmbedDownloadInfo, fastembed_model_download_info};

use crate::AppState;

#[cfg(feature = "local-embedding")]
use super::super::contracts::EmbeddingModelDownloadFileSummary;
use super::super::{
    contracts::{EmbeddingModelDownloadInfoSummary, EmbeddingModelExecution, EmbeddingModelInput},
    host_adapter::{embedding_model_execution, required_text},
};

pub async fn get_embedding_model_download_info(
    state: &AppState,
    input: EmbeddingModelInput,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    if embedding_model_execution(state.runtime_profile()) == EmbeddingModelExecution::Sidecar {
        return crate::sidecar_clients::embedding_model_operation(
            state.capabilities(),
            "download_info",
            &model,
        )
        .await;
    }
    get_embedding_model_download_info_in_cache(&model, &state.default_fastembed_cache_dir()).await
}

pub async fn get_embedding_model_download_info_in_cache(
    model: &str,
    cache_dir: &Path,
) -> Result<EmbeddingModelDownloadInfoSummary, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let model = required_text(model, "fastembed 模型名")?;
        let info = fastembed_model_download_info(&model, cache_dir.to_path_buf()).await?;
        Ok(to_embedding_model_download_info_summary(info))
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (model, cache_dir);
        Err(super::super::provider::local_embedding_disabled_error())
    }
}

#[cfg(feature = "local-embedding")]
fn to_embedding_model_download_info_summary(
    info: FastEmbedDownloadInfo,
) -> EmbeddingModelDownloadInfoSummary {
    EmbeddingModelDownloadInfoSummary {
        model_name: info.model_name,
        model_code: info.model_code,
        total_size_bytes: info.total_size_bytes,
        downloaded_size_bytes: info.downloaded_size_bytes,
        missing_size_bytes: info.missing_size_bytes,
        files: info
            .files
            .into_iter()
            .map(|file| EmbeddingModelDownloadFileSummary {
                path: file.path,
                size_bytes: file.size_bytes,
                downloaded_size_bytes: file.downloaded_size_bytes,
                downloaded: file.downloaded,
            })
            .collect(),
    }
}
