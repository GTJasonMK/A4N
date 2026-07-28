use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError, ServiceErrorCode,
    redact_sensitive_message,
};
use serde_json::{Value, json};

use crate::contracts::ImageGenerationOutput;

pub(crate) fn progress_generation_event(task_id: String, sequence: u64) -> CapabilityTaskEvent {
    image_generation_event(
        task_id,
        CapabilityTaskEventKind::Progress,
        sequence,
        None,
        Some(json!({
            "current": 1,
            "total": 2,
            "message": "image sidecar 正在调用 provider"
        })),
    )
}

pub(crate) fn completed_generation_event(
    task_id: String,
    sequence: u64,
    output: ImageGenerationOutput,
) -> Result<CapabilityTaskEvent, ServiceError> {
    let payload = serde_json::to_value(output).map_err(|error| ServiceError {
        code: ServiceErrorCode::Internal,
        message: redact_sensitive_message(format!(
            "image generation output serialization failed: {error}"
        )),
        retryable: Some(false),
    })?;
    Ok(image_generation_event(
        task_id,
        CapabilityTaskEventKind::Completed,
        sequence,
        None,
        Some(payload),
    ))
}

pub(crate) fn failed_generation_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> CapabilityTaskEvent {
    image_generation_event(
        task_id,
        CapabilityTaskEventKind::Failed,
        sequence,
        Some(error),
        None,
    )
}

fn image_generation_event(
    task_id: String,
    event_kind: CapabilityTaskEventKind,
    sequence: u64,
    error: Option<ServiceError>,
    payload: Option<Value>,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
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
