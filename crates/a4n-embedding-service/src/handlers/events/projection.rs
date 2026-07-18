use a4n_embedding::provider::DownloadProgress;
use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceTaskEvent, ServiceTaskEventKind,
    redact_sensitive_message,
};

pub(crate) fn download_progress_event(
    task_id: String,
    sequence: u64,
    progress: DownloadProgress,
) -> Result<ServiceTaskEvent, ServiceError> {
    let payload = serde_json::to_value(progress).map_err(|error| ServiceError {
        code: ServiceErrorCode::Internal,
        message: redact_sensitive_message(format!(
            "embedding download progress serialization failed: {error}"
        )),
        retryable: Some(false),
    })?;
    Ok(ServiceTaskEvent {
        task_id,
        event_kind: ServiceTaskEventKind::Progress,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: Some(payload),
    })
}

pub(crate) fn completed_download_event(
    task_id: String,
    sequence: u64,
    models: serde_json::Value,
) -> ServiceTaskEvent {
    ServiceTaskEvent {
        task_id,
        event_kind: ServiceTaskEventKind::Completed,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: Some(serde_json::json!({ "models": models })),
    }
}

pub(crate) fn failed_download_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> ServiceTaskEvent {
    ServiceTaskEvent {
        task_id,
        event_kind: ServiceTaskEventKind::Failed,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage: None,
        finish_reason: None,
        is_truncated_finish_reason: None,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: Some(error),
        payload: None,
    }
}
