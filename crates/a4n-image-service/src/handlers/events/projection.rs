use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError};
use serde_json::json;

use crate::contracts::ImageGenerationOutput;

pub(crate) fn progress_generation_event(task_id: String, sequence: u64) -> CapabilityTaskEvent {
    CapabilityTaskEvent::new(task_id, CapabilityTaskEventKind::Progress, sequence).with_payload(
        json!({
            "current": 1,
            "total": 2,
            "message": "image sidecar 正在调用 provider"
        }),
    )
}

pub(crate) fn completed_generation_event(
    task_id: String,
    sequence: u64,
    output: ImageGenerationOutput,
) -> Result<CapabilityTaskEvent, ServiceError> {
    CapabilityTaskEvent::new(task_id, CapabilityTaskEventKind::Completed, sequence)
        .with_serialized_payload(output, "image generation output serialization failed")
}

pub(crate) fn failed_generation_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent::failed(task_id, sequence, error)
}
