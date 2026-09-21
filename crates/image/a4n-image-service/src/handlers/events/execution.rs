use std::convert::Infallible;

use a4n_image_providers::pick_image_provider;
use a4n_service_protocol::{CapabilityTaskEvent, ServiceRequest};
use a4n_sidecar_bootstrap::service_event_channel;
use axum::body::Bytes;
use base64::Engine as _;
use futures::Stream;
use std::time::Duration;

use crate::contracts::{ImageGenerationOutput, ImageProviderPayload};
use crate::protocol::service_error;

const IMAGE_PROGRESS_HEARTBEAT: Duration = Duration::from_secs(5);

pub(super) fn image_generation_event_stream(
    request: ServiceRequest<ImageProviderPayload>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_stream) = service_event_channel();
    let task_id = request.request_id.clone();
    tokio::spawn(async move {
        if event_tx
            .send(CapabilityTaskEvent::progress_message(
                task_id.clone(),
                1,
                "image sidecar 正在调用 provider",
            ))
            .await
            .is_err()
        {
            return;
        }
        let provider_task = async move {
            let provider = pick_image_provider(
                &request.payload.backend,
                &request.payload.base_url,
                &request.payload.api_key,
            )?;
            provider
                .generate(request.payload.request.clone())
                .await
                .map(|result| ImageGenerationOutput {
                    bytes_base64: base64::engine::general_purpose::STANDARD.encode(result.bytes),
                    format: result.format,
                    model: result.model,
                })
        };
        tokio::pin!(provider_task);
        let mut sequence = 2;
        let mut heartbeat = tokio::time::interval(IMAGE_PROGRESS_HEARTBEAT);
        heartbeat.tick().await;
        loop {
            tokio::select! {
                _ = event_tx.closed() => break,
                _ = heartbeat.tick() => {
                    if event_tx.send(CapabilityTaskEvent::progress_message(
                        task_id.clone(),
                        sequence,
                        "image sidecar 正在等待 provider 完成",
                    )).await.is_err() {
                        break;
                    }
                    sequence += 1;
                }
                result = &mut provider_task => {
                    let event = match result {
                        Ok(output) => CapabilityTaskEvent::completed_or_failed(
                            task_id.clone(),
                            sequence,
                            output,
                            "image generation output serialization failed",
                        ),
                        Err(error) => CapabilityTaskEvent::failed(task_id.clone(), sequence, service_error(error)),
                    };
                    let _ = event_tx.send(event).await;
                    break;
                }
            }
        }
    });
    event_stream
}
