//! 视频生成 Provider trait + 通用 HTTP 实现
//!
//! V1 支持三类 HTTP 合同：
//! - 同步返回 `data[0].b64_json` 或 `data[0].url`，并必须带 `data[0].format`
//! - 异步返回顶层 `id`，轮询响应必须带顶层 `status`
//! - New API 单数 `/video/generations`，适配 `grok-imagine-video` 等视频模型

mod chat;
mod cliproxy_openai;
mod download;
mod factory;
mod format;
mod gemini_veo;
mod http;
mod newapi;
mod response;
mod seedance2;
mod types;

pub use chat::ChatCompletionsVideoProvider;
pub use cliproxy_openai::CliProxyOpenAiVideoProvider;
pub use factory::pick_video_provider;
pub use gemini_veo::GeminiVeoVideoProvider;
pub use http::HttpVideoProvider;
pub use newapi::NewApiVideoProvider;
pub use seedance2::Seedance2VideoProvider;
pub use types::{
    VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider,
    VideoReferenceMediaRequest,
};
