use a4n_core::error::AppError;
use a4n_service_protocol::CapabilityTaskEvent;

use crate::sidecar_clients::{GenerationEvent, parse_generation_event, service_app_error};

pub(super) fn handle_rag_ingest_event(
    event: CapabilityTaskEvent,
) -> Result<Option<usize>, AppError> {
    match parse_generation_event(event, "rag").map_err(service_app_error)? {
        GenerationEvent::Progress(_) => Ok(None),
        GenerationEvent::Completed(output) => Ok(Some(output)),
    }
}
