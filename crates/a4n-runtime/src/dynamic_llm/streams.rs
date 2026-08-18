#[cfg(feature = "embedded-llm-provider")]
mod embedded;
mod events;
mod sidecar;
mod terminal;

#[cfg(feature = "embedded-llm-provider")]
pub(super) use embedded::QueueTrackedLlmStream;
pub(super) use events::{SidecarLlmEvent, parse_sidecar_llm_event};
pub(super) use sidecar::QueueTrackedSidecarLlmStream;
