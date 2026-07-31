use a4n_embedding::provider::DownloadProgress;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError};

pub(crate) fn download_progress_event(
    task_id: String,
    sequence: u64,
    progress: DownloadProgress,
) -> Result<CapabilityTaskEvent, ServiceError> {
    CapabilityTaskEvent::new(task_id, CapabilityTaskEventKind::Progress, sequence)
        .with_serialized_payload(progress, "embedding download progress serialization failed")
}

pub(crate) fn completed_download_event(
    task_id: String,
    sequence: u64,
    models: serde_json::Value,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent::new(task_id, CapabilityTaskEventKind::Completed, sequence)
        .with_payload(serde_json::json!({ "models": models }))
}

pub(crate) fn failed_download_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent::failed(task_id, sequence, error)
}
