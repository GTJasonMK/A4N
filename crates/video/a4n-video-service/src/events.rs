use crate::protocol::service_error;
use crate::providers::video_generation_output;
use a4n_service_protocol::{CapabilityTaskEvent, ServiceRequest};
use a4n_sidecar_bootstrap::service_event_channel;
use a4n_video_providers::{materialize_video_input, pick_video_provider};
use a4n_video_sidecar_contract::VideoProviderPayload;
use axum::body::Bytes;
use futures::Stream;
use std::convert::Infallible;

pub(crate) fn video_generation_event_stream(
    request: ServiceRequest<VideoProviderPayload>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_stream) = service_event_channel();
    let task_id = request.request_id.clone();
    tokio::spawn(async move {
        if event_tx
            .send(CapabilityTaskEvent::progress_message(
                task_id.clone(),
                1,
                "video sidecar 正在提交 provider 请求",
            ))
            .await
            .is_err()
        {
            return;
        }
        let provider_task = async move {
            let provider = pick_video_provider(
                &request.payload.backend,
                &request.payload.base_url,
                &request.payload.api_key,
            )?;
            let input = materialize_video_input(
                request.payload.request,
                std::env::var_os("A4N_DATA_DIR").map(std::path::PathBuf::from),
            )
            .await?;
            provider
                .generate(
                    input,
                    request.payload.provider_idempotency_key_sha256.as_deref(),
                )
                .await
                .map(video_generation_output)
        };
        tokio::pin!(provider_task);
        tokio::select! {
            _ = event_tx.closed() => {},
            result = &mut provider_task => {
                let event = match result {
                    Ok(output) => CapabilityTaskEvent::completed_or_failed(
                        task_id.clone(),
                        2,
                        output,
                        "video generation output serialization failed",
                    ),
                    Err(error) => CapabilityTaskEvent::failed(task_id, 2, service_error(error)),
                };
                let _ = event_tx.send(event).await;
            }
        }
    });
    event_stream
}
