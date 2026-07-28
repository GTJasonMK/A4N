#[cfg(feature = "embedded-llm-provider")]
mod embedded;
mod events;
mod sidecar;
mod terminal;

#[cfg(feature = "embedded-llm-provider")]
pub(super) use embedded::QueueTrackedLlmStream;
pub(super) use events::{chat_chunk_from_sidecar_event, sidecar_event_error, sidecar_failed_event};
pub(super) use sidecar::QueueTrackedSidecarLlmStream;
