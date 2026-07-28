use std::{convert::Infallible, path::PathBuf};

use a4n_core::error::AppError;
use a4n_embedding::provider::DownloadProgress;
use a4n_embedding_fastembed::{download_fastembed_model, list_fastembed_local_models};
use a4n_service_protocol::{
    CapabilityTaskEvent, ServiceError, ServiceErrorCode, redact_sensitive_message,
};
use a4n_sidecar_bootstrap::service_event_line;
use axum::body::Bytes;
use futures::{Stream, stream};
use tokio_util::sync::CancellationToken;

use super::projection::{completed_download_event, download_progress_event, failed_download_event};
use crate::protocol::service_error;

pub(super) fn embedding_download_event_stream(
    task_id: String,
    model: String,
    cache_dir: PathBuf,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<CapabilityTaskEvent>(
        a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY,
    );
    tokio::spawn(async move {
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::unbounded_channel::<DownloadProgress>();
        let download_cache_dir = cache_dir.clone();
        let cancel_token = CancellationToken::new();
        let download_cancel_token = cancel_token.clone();
        let mut download_task = tokio::spawn(async move {
            download_fastembed_model(
                &model,
                download_cache_dir.clone(),
                Some(progress_tx),
                Some(download_cancel_token),
            )
            .await?;
            let models = list_fastembed_local_models(&download_cache_dir)?;
            serde_json::to_value(models).map_err(AppError::from)
        });
        let mut sequence = 0;
        let mut progress_open = true;
        loop {
            tokio::select! {
                _ = event_tx.closed() => {
                    cancel_token.cancel();
                    let _ = download_task.await;
                    break;
                }
                progress = progress_rx.recv(), if progress_open => match progress {
                    Some(progress) => {
                        sequence += 1;
                        let event = match download_progress_event(task_id.clone(), sequence, progress) {
                            Ok(event) => event,
                            Err(error) => {
                                cancel_token.cancel();
                                if event_tx.send(failed_download_event(task_id.clone(), sequence, error)).await.is_err() {
                                    let _ = download_task.await;
                                    break;
                                }
                                let _ = download_task.await;
                                break;
                            }
                        };
                        if event_tx.send(event).await.is_err() {
                            cancel_token.cancel();
                            let _ = download_task.await;
                            break;
                        }
                    }
                    None => progress_open = false,
                },
                result = &mut download_task => {
                    while let Ok(progress) = progress_rx.try_recv() {
                        sequence += 1;
                        let event = match download_progress_event(task_id.clone(), sequence, progress) {
                            Ok(event) => event,
                            Err(error) => {
                                    if event_tx.send(failed_download_event(task_id.clone(), sequence, error)).await.is_err() {
                                    return;
                                }
                                return;
                            }
                        };
                        if event_tx.send(event).await.is_err() {
                            return;
                        }
                    }
                    sequence += 1;
                    let event = match result {
                        Ok(Ok(models)) => completed_download_event(task_id.clone(), sequence, models),
                        Ok(Err(error)) => failed_download_event(task_id.clone(), sequence, service_error(&error)),
                        Err(error) => failed_download_event(
                            task_id.clone(),
                            sequence,
                            ServiceError {
                                code: ServiceErrorCode::Internal,
                                message: redact_sensitive_message(format!(
                                    "embedding download task failed: {error}"
                                )),
                                retryable: Some(false),
                            },
                        ),
                    };
                    if event_tx.send(event).await.is_err() {
                        break;
                    }
                    break;
                }
            }
        }
    });
    stream::unfold(event_rx, |mut event_rx| async move {
        event_rx
            .recv()
            .await
            .map(|event| (Ok(service_event_line(event)), event_rx))
    })
}
