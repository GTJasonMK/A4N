//! Concrete video provider implementations.
//!
//! Host minimal builds should depend on `a4n-video` contracts, not this crate.

mod chat;
mod cliproxy_openai;
mod download;
mod factory;
mod format;
mod gemini_veo;
mod http;
mod idempotency;
mod newapi;
mod response;
mod seedance2;
mod transport;

pub use factory::{pick_video_provider, video_provider_request_body_preview};
