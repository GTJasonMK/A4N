use a4n_llm::provider::{ChatChunk, UsageInfo, is_truncated_finish_reason};
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind, ServiceError};

use crate::protocol::service_error;

pub(super) fn chunk_event(
    task_id: String,
    sequence: u64,
    chunk: ChatChunk,
) -> Result<CapabilityTaskEvent, ServiceError> {
    let usage = chunk.usage.as_ref().map(usage_value).transpose()?;
    let payload = serde_json::to_value(&chunk).map_err(|error| service_error(error.into()))?;
    let finish_reason = chunk.finish_reason;

    Ok(CapabilityTaskEvent {
        task_id,
        event_kind: CapabilityTaskEventKind::Chunk,
        sequence,
        chunk_index: Some(u64::from(chunk.index)),
        chunk_delta: chunk.content.or(chunk.reasoning_content),
        usage_delta: usage,
        final_usage: None,
        is_truncated_finish_reason: finish_reason
            .as_deref()
            .map(|reason| is_truncated_finish_reason(Some(reason))),
        finish_reason,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: Some(payload),
    })
}

pub(super) fn completed_event(
    task_id: String,
    sequence: u64,
    final_usage: Option<serde_json::Value>,
    finish_reason: Option<String>,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id,
        event_kind: CapabilityTaskEventKind::Completed,
        sequence,
        chunk_index: None,
        chunk_delta: None,
        usage_delta: None,
        final_usage,
        is_truncated_finish_reason: Some(is_truncated_finish_reason(finish_reason.as_deref())),
        finish_reason,
        checkpoint_key: None,
        artifact_refs: Vec::new(),
        error: None,
        payload: None,
    }
}

pub(super) fn failed_event(
    task_id: String,
    sequence: u64,
    error: ServiceError,
) -> CapabilityTaskEvent {
    CapabilityTaskEvent {
        task_id,
        event_kind: CapabilityTaskEventKind::Failed,
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

fn usage_value(usage: &UsageInfo) -> Result<serde_json::Value, ServiceError> {
    serde_json::to_value(usage).map_err(|error| service_error(error.into()))
}
