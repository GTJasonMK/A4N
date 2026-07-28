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

pub use chat::ChatCompletionsVideoProvider;
pub use cliproxy_openai::CliProxyOpenAiVideoProvider;
pub use factory::pick_video_provider;
pub use gemini_veo::GeminiVeoVideoProvider;
pub use http::HttpVideoProvider;
pub use newapi::NewApiVideoProvider;
pub use seedance2::Seedance2VideoProvider;
