//! Concrete image provider implementations.
//!
//! Host minimal builds should depend on `a4n-image` contracts, not this crate.

mod chat;
mod cliproxy;
mod comfyui;
mod factory;
mod openai;
mod openai_edit;
mod sd_webui;
mod support;

pub use factory::{image_provider_request_body_preview, pick_image_provider};
