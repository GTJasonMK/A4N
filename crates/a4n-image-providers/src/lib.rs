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

pub use chat::ChatCompletionsImageProvider;
pub use cliproxy::CliProxyImageProvider;
pub use comfyui::ComfyUiImageProvider;
pub use factory::pick_image_provider;
pub use openai::OpenAiCompatibleImageProvider;
pub use openai_edit::OpenAiCompatibleImageEditProvider;
pub use sd_webui::StableDiffusionWebUiImageProvider;
