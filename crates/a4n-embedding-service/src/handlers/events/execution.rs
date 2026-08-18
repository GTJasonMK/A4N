use std::{convert::Infallible, path::PathBuf};

use a4n_embedding::provider::DownloadProgress;
use a4n_embedding_fastembed::download_fastembed_model;
use a4n_service_protocol::CapabilityTaskEvent;
use a4n_sidecar_bootstrap::service_event_channel;
use axum::body::Bytes;
use futures::Stream;
use tokio_util::sync::CancellationToken;

use crate::protocol::service_error;

pub(super) fn embedding_download_event_stream(
    task_id: String,
    model: String,
    cache_dir: PathBuf,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_stream) = service_event_channel();
    tokio::spawn(async move {
        let (progress_tx, mut progress_rx) =
            tokio::sync::mpsc::unbounded_channel::<DownloadProgress>();
        let cancel_token = CancellationToken::new();
        let download_cancel_token = cancel_token.clone();
        let download_task = async move {
            download_fastembed_model(
                &model,
                cache_dir,
                Some(progress_tx),
                download_cancel_token,
            )
            .await
        };
        tokio::pin!(download_task);
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
                        let event = match CapabilityTaskEvent::progress_with_payload(
                            task_id.clone(),
                            sequence,
                            progress,
                            "embedding download progress serialization failed",
                        ) {
                            Ok(event) => event,
                            Err(error) => {
                                cancel_token.cancel();
                                let _ = event_tx
                                    .send(CapabilityTaskEvent::failed(
                                        task_id.clone(),
                                        sequence,
                                        error,
                                    ))
                                    .await;
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
                        let event = match CapabilityTaskEvent::progress_with_payload(
                            task_id.clone(),
                            sequence,
                            progress,
                            "embedding download progress serialization failed",
                        ) {
                            Ok(event) => event,
                            Err(error) => {
                                let _ = event_tx
                                    .send(CapabilityTaskEvent::failed(
                                        task_id.clone(),
                                        sequence,
                                        error,
                                    ))
                                    .await;
                                return;
                            }
                        };
                        if event_tx.send(event).await.is_err() {
                            return;
                        }
                    }
                    sequence += 1;
                    let event = match result {
                        Ok(model) => CapabilityTaskEvent::completed_or_failed(
                            task_id.clone(),
                            sequence,
                            model,
                            "embedding model serialization failed",
                        ),
                        Err(error) => CapabilityTaskEvent::failed(
                            task_id.clone(),
                            sequence,
                            service_error(error),
                        ),
                    };
                    let _ = event_tx.send(event).await;
                    break;
                }
            }
        }
    });
    event_stream
}
