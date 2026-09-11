use a4n_error::AppError;
use a4n_llm::provider::ChatChunk;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};

use super::super::config::llm_sidecar_error;

pub(in crate::dynamic_llm) enum SidecarLlmEvent {
    Chunk(ChatChunk),
    Progress,
    Completed,
}

pub(in crate::dynamic_llm) fn parse_sidecar_llm_event(
    event: CapabilityTaskEvent,
) -> Result<SidecarLlmEvent, AppError> {
    match event.event_kind {
        CapabilityTaskEventKind::Chunk => {
            let payload = event.payload.ok_or_else(|| {
                AppError::LlmError("LLM sidecar chunk 事件缺少 payload".to_string())
            })?;
            serde_json::from_value(payload)
                .map(SidecarLlmEvent::Chunk)
                .map_err(AppError::Serialization)
        }
        CapabilityTaskEventKind::Completed => Ok(SidecarLlmEvent::Completed),
        CapabilityTaskEventKind::Failed => {
            let error = event.error.ok_or_else(|| {
                AppError::LlmError("LLM sidecar failed 事件缺少 error".to_string())
            })?;
            Err(llm_sidecar_error(error))
        }
        CapabilityTaskEventKind::Progress => Ok(SidecarLlmEvent::Progress),
    }
}
