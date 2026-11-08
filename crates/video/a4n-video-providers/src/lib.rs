//! Concrete video provider implementations.
//!
//! Host minimal builds should depend on `a4n-video` contracts, not this crate.

mod chat;
mod cliproxy_openai;
#[cfg(test)]
mod contract_matrix;
mod download;
mod factory;
mod format;
mod gemini_veo;
mod http;
mod idempotency;
mod media;
mod newapi;
mod response;
mod seedance2;
mod transport;

pub use factory::{pick_video_provider, video_provider_request_body_preview};
pub use media::materialize_video_input;
