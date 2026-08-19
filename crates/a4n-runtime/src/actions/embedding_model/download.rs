use super::{
    DownloadProgress, EmbeddingModelInput, LocalEmbeddingModelSummary, host_adapter::required_text,
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
    cancel_token: CancellationToken,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    let model = required_text(&input.model, "fastembed 模型名")?;
    if state.runtime_profile().uses_sidecars() {
        let capabilities = state.capabilities().clone();
        let queue_cancel_token = cancel_token.clone();
        return state
            .orchestrator()
            .queue_run_tracked_with_cancel(
                QueueTaskKind::EmbeddingDownload,
                TaskPriority::Low,
                None,
                queue_cancel_token,
                || async move {
                    crate::sidecar_clients::download_embedding_model_events(
                        &capabilities,
                        &model,
                        progress_tx,
                        cancel_token,
                    )
                    .await
                },
            )
            .await;
    }

    #[cfg(feature = "local-embedding")]
    {
        let cache_dir = state.embedding_models_dir();
        let queue_cancel_token = cancel_token.clone();
        state
            .orchestrator()
            .queue_run_tracked_with_cancel(
                QueueTaskKind::EmbeddingDownload,
                TaskPriority::Low,
                None,
                queue_cancel_token,
                || async move {
                    download_fastembed_model(&model, cache_dir, progress_tx, cancel_token).await
                },
            )
            .await
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
    cancel_token: CancellationToken,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    #[cfg(feature = "local-embedding")]
    {
        let model = required_text(&input.model, "fastembed 模型名")?;
        download_fastembed_model(&model, cache_dir, progress_tx, cancel_token).await
    }
    #[cfg(not(feature = "local-embedding"))]
    {
        let _ = (input, cache_dir, progress_tx, cancel_token);
        Err(super::provider::local_embedding_disabled_error())
    }
}
