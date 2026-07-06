//! 图像生成 Provider trait + HTTP 后端实现。
//!
//! 用于 ImageService / commands::image::generate_* 调用。

mod chat;
mod cliproxy;
mod comfyui;
mod factory;
mod openai;
mod openai_edit;
mod sd_webui;
mod support;
mod types;

pub use chat::ChatCompletionsImageProvider;
pub use cliproxy::CliProxyImageProvider;
pub use comfyui::ComfyUiImageProvider;
pub use factory::pick_image_provider;
pub use openai::OpenAiCompatibleImageProvider;
pub use openai_edit::OpenAiCompatibleImageEditProvider;
pub use sd_webui::StableDiffusionWebUiImageProvider;
pub use types::{ImageGenerationRequest, ImageGenerationResult, ImageProvider, ImageReference};
