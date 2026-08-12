#[cfg(feature = "local-embedding")]
use super::models::{list_local_embedding_models, list_local_embedding_models_in_cache};
use super::{
    DownloadProgress, EmbeddingModelInput, LocalEmbeddingModelSummary,
    host_adapter::{downloaded_model_summary, required_text},
};
use crate::AppState;
use a4n_core::error::AppError;
#[cfg(feature = "local-embedding")]
use a4n_embedding_fastembed::download_fastembed_model;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;

pub async fn download_embedding_model(
    state: &AppState,
    input: EmbeddingModelInput,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    if state.runtime_profile().uses_sidecars() {
        let capabilities = state.capabilities().clone();
        let model_for_download = model.clone();
        let queue_cancel_token = cancel_token.clone().unwrap_or_default();
        let models = state
            .orchestrator()
            .queue_run_tracked_with_cancel(
                QueueTaskKind::EmbeddingDownload,
                TaskPriority::Low,
                None,
                queue_cancel_token,
                || async move {
                    crate::sidecar_clients::download_embedding_model_events(
                        &capabilities,
                        &model_for_download,
                        progress_tx,
                        cancel_token,
                    )
                    .await
                },
            )
            .await?;
        let models: Vec<LocalEmbeddingModelSummary> =
            serde_json::from_value(models).map_err(|error| {
                AppError::ConfigError(format!(
                    "embedding sidecar 下载完成模型列表解析失败: {error}"
                ))
            })?;
        return downloaded_model_summary(models, &model);
    }

    #[cfg(feature = "local-embedding")]
    {
        let cache_dir = state.default_fastembed_cache_dir();
        let queue_cancel_token = cancel_token.clone().unwrap_or_default();
        let model_for_download = model.clone();
        let cache_dir_for_download = cache_dir.clone();

        let download_result = state
            .orchestrator()
            .queue_run_tracked_with_cancel(
                QueueTaskKind::EmbeddingDownload,
                TaskPriority::Low,
                None,
                queue_cancel_token,
                || async move {
                    download_fastembed_model(
                        &model_for_download,
                        cache_dir_for_download,
                        progress_tx,
                        cancel_token,
                    )
                    .await
                },
            )
            .await;
        download_result?;

        downloaded_model_summary(list_local_embedding_models(state).await?, &model)
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (state, progress_tx, cancel_token);
        Err(super::provider::local_embedding_disabled_error())
    }
}

pub async fn download_embedding_model_to_cache(
    input: EmbeddingModelInput,
    cache_dir: PathBuf,
    progress_tx: Option<UnboundedSender<DownloadProgress>>,
    cancel_token: Option<CancellationToken>,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let model = required_text(&input.model, "fastembed 模型名")?;
        download_fastembed_model(&model, cache_dir.clone(), progress_tx, cancel_token).await?;
        downloaded_model_summary(list_local_embedding_models_in_cache(&cache_dir)?, &model)
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (input, cache_dir, progress_tx, cancel_token);
        Err(super::provider::local_embedding_disabled_error())
    }
}
