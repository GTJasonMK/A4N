use crate::contracts::{VideoGenerationOutput, VideoProviderPayload};
use crate::protocol::service_error;
use crate::providers::video_generation_output;
use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceTaskEvent, ServiceTaskEventKind,
    redact_sensitive_message,
};
use a4n_sidecar_bootstrap::service_event_line;
use a4n_video_providers::pick_video_provider;
use axum::body::Bytes;
use futures::{Stream, stream};
use serde_json::{Value, json};
use std::convert::Infallible;

pub(crate) fn video_generation_event_stream(
    request: ServiceRequest<VideoProviderPayload>,
) -> impl Stream<Item = Result<Bytes, Infallible>> + Send + 'static {
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<ServiceTaskEvent>();
    let task_id = request.request_id.clone();
    tokio::spawn(async move {
        if event_tx
            .send(progress_generation_event(task_id.clone(), 1))
            .is_err()
        {
            return;
        }
        let mut provider_task = tokio::spawn(async move {
            let provider = pick_video_provider(
                &request.payload.backend,
                &request.payload.base_url,
                &request.payload.api_key,
                request.payload.provider_options_json.as_ref(),
            )?;
            provider
                .generate(request.payload.request.clone())
                .await
                .map(video_generation_output)
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
                        ServiceError {
                            code: ServiceErrorCode::Internal,
                            message: redact_sensitive_message(format!(
                                "video generation task failed: {error}"
                            )),
                            retryable: Some(false),
                        },
                    ),
                };
                if event_tx.send(event).is_err() {
                    return;
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

pub(crate) fn progress_generation_event(task_id: String, sequence: u64) -> ServiceTaskEvent {
    video_generation_event(
        task_id,
        ServiceTaskEventKind::Progress,
        sequence,
        None,
        Some(json!({
            "current": 1,
            "total": 2,
            "message": "video sidecar 正在提交 provider 请求"
        })),
    )
}

pub(crate) fn completed_generation_event(
    task_id: String,
    sequence: u64,
    output: VideoGenerationOutput,
) -> Result<ServiceTaskEvent, ServiceError> {
    let payload = serde_json::to_value(output).map_err(|error| ServiceError {
        code: ServiceErrorCode::Internal,
        message: redact_sensitive_message(format!(
            "video generation output serialization failed: {error}"
        )),
        retryable: Some(false),
    })?;
    Ok(video_generation_event(
        task_id,
        ServiceTaskEventKind::Completed,
        sequence,
        None,
        Some(payload),
    ))
}

pub(crate) fn failed_generation_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> ServiceTaskEvent {
    video_generation_event(
        task_id,
        ServiceTaskEventKind::Failed,
        sequence,
        Some(error),
        None,
    )
}

pub(crate) fn video_generation_event(
    task_id: String,
    event_kind: ServiceTaskEventKind,
    sequence: u64,
    error: Option<ServiceError>,
    payload: Option<Value>,
) -> ServiceTaskEvent {
    ServiceTaskEvent {
        task_id,
        event_kind,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error,
        payload,
    }
}
