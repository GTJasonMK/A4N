use std::convert::Infallible;

use a4n_core::error::AppError;
use a4n_image_providers::pick_image_provider;
use a4n_service_protocol::{
    CapabilityTaskEvent, ServiceError, ServiceErrorCode, ServiceRequest, redact_sensitive_message,
};
use a4n_sidecar_bootstrap::service_event_line;
use axum::body::Bytes;
use base64::Engine as _;
use futures::{Stream, stream};

use super::projection::{
    completed_generation_event, failed_generation_event, progress_generation_event,
};
use crate::contracts::{ImageGenerationOutput, ImageProviderPayload};
use crate::protocol::service_error;

pub(super) fn image_generation_event_stream(
    request: ServiceRequest<ImageProviderPayload>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_rx) = tokio::sync::mpsc::channel::<CapabilityTaskEvent>(
        a4n_service_protocol::SIDECAR_EVENT_QUEUE_CAPACITY,
    );
    let task_id = request.request_id.clone();
    tokio::spawn(async move {
        if event_tx
            .send(progress_generation_event(task_id.clone(), 1))
            .await
            .is_err()
        {
            return;
        }
        let mut provider_task = tokio::spawn(async move {
            let provider = pick_image_provider(
                &request.payload.backend,
                &request.payload.base_url,
                &request.payload.api_key,
            )?;
            let result = provider.generate(request.payload.request.clone()).await?;
            Ok::<_, AppError>(ImageGenerationOutput {
                bytes_base64: base64::engine::general_purpose::STANDARD.encode(result.bytes),
                format: result.format,
                model: result.model,
            })
        });
        tokio::select! {
            _ = event_tx.closed() => provider_task.abort(),
            result = &mut provider_task => {
                let event = match result {
                    Ok(Ok(output)) => match completed_generation_event(task_id.clone(), 2, output) {
                        Ok(event) => event,
                        Err(error) => failed_generation_event(task_id, 2, error),
                    },
                    Ok(Err(error)) => failed_generation_event(task_id, 2, service_error(&error)),
                    Err(error) => failed_generation_event(
                        task_id,
                        2,
                        ServiceError::non_retryable(
                            ServiceErrorCode::Internal,
                            redact_sensitive_message(format!(
                                "image generation task failed: {error}"
                            )),
                        ),
                    ),
                };
                if event_tx.send(event).await.is_err() {
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
