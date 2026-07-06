use a4n_core::error::AppError;
use serde_json::Value;

use super::chat::ChatCompletionsVideoProvider;
use super::cliproxy_openai::CliProxyOpenAiVideoProvider;
use super::gemini_veo::GeminiVeoVideoProvider;
use super::http::HttpVideoProvider;
use super::newapi::NewApiVideoProvider;
use super::seedance2::Seedance2VideoProvider;
use super::types::VideoProvider;

pub fn pick_video_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
    provider_options_json: Option<&Value>,
) -> Result<Box<dyn VideoProvider>, AppError> {
    match backend {
        "openai-compatible-video" => Ok(Box::new(HttpVideoProvider::new(base_url, api_key))),
        "newapi-video" => Ok(Box::new(NewApiVideoProvider::new(base_url, api_key))),
        "gemini-veo-video" => Ok(Box::new(GeminiVeoVideoProvider::new(base_url, api_key))),
        "seedance2-video" => Ok(Box::new(Seedance2VideoProvider::new(base_url, api_key))),
        "cliproxy-openai-video" => Ok(Box::new(CliProxyOpenAiVideoProvider::new_with_options(
            base_url,
            api_key,
            provider_options_json,
        )?)),
        "openai-chat-video" => Ok(Box::new(ChatCompletionsVideoProvider::new(
            base_url, api_key,
        )?)),
        other => Err(AppError::Internal(anyhow::anyhow!(
            "未知视频 backend={other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_video_backends() {
        for backend in [
            "openai-compatible-video",
            "newapi-video",
            "gemini-veo-video",
            "seedance2-video",
            "cliproxy-openai-video",
            "openai-chat-video",
        ] {
            assert!(
                pick_video_provider(backend, "http://localhost:3000", "test-key", None).is_ok(),
                "backend should be accepted: {backend}"
            );
        }
    }

    #[test]
    fn rejects_unknown_video_backend() {
        assert!(
            pick_video_provider(
                "unsupported-video-backend",
                "http://localhost:3000",
                "test-key",
                None,
            )
            .is_err()
        );
    }

    #[test]
    fn chat_video_backend_requires_base_url() {
        assert!(pick_video_provider("openai-chat-video", "", "test-key", None).is_err());
    }

    #[test]
    fn chat_video_backend_requires_api_key() {
        assert!(
            pick_video_provider("openai-chat-video", "http://localhost:3000", "", None).is_err()
        );
    }
}
