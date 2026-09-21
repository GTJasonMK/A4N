use a4n_error::AppError;
use a4n_service_protocol::{CapabilityTaskEvent, CapabilityTaskEventKind};

use crate::sidecar_clients::{
    GenerationEvent, parse_generation_event, service_app_error, service_error_app_error,
};

pub(super) fn handle_rag_ingest_event(
    event: CapabilityTaskEvent,
) -> Result<Option<usize>, AppError> {
    if event.event_kind == CapabilityTaskEventKind::Failed {
        let error = event
            .error
            .ok_or_else(|| service_app_error("rag sidecar failed 事件缺少 error"))?;
        return Err(service_error_app_error(error));
    }

    match parse_generation_event(event, "rag").map_err(service_app_error)? {
        GenerationEvent::Progress(_) => Ok(None),
        GenerationEvent::Completed(output) => Ok(Some(output)),
    }
}
