use a4n_core::error::AppError;

use super::chat::ChatCompletionsVideoProvider;
use super::cliproxy_openai::CliProxyOpenAiVideoProvider;
use super::gemini_veo::GeminiVeoVideoProvider;
use super::http::HttpVideoProvider;
use super::newapi::NewApiVideoProvider;
use super::seedance2::Seedance2VideoProvider;
use a4n_video::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
    VideoProvider, video_provider_cleanup_capabilities,
};

pub fn pick_video_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Box<dyn VideoProvider>, AppError> {
    if video_provider_cleanup_capabilities(backend).is_none() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "未知视频 backend={backend}"
        )));
    }
    match backend {
        OPENAI_COMPATIBLE_VIDEO_BACKEND => Ok(Box::new(HttpVideoProvider::new(base_url, api_key))),
        NEWAPI_VIDEO_BACKEND => Ok(Box::new(NewApiVideoProvider::new(base_url, api_key))),
        GEMINI_VEO_VIDEO_BACKEND => Ok(Box::new(GeminiVeoVideoProvider::new(base_url, api_key))),
        SEEDANCE2_VIDEO_BACKEND => Ok(Box::new(Seedance2VideoProvider::new(base_url, api_key))),
        CLIPROXY_OPENAI_VIDEO_BACKEND => Ok(Box::new(CliProxyOpenAiVideoProvider::new(
            base_url, api_key,
        ))),
        OPENAI_CHAT_VIDEO_BACKEND => Ok(Box::new(ChatCompletionsVideoProvider::new(
            base_url, api_key,
        )?)),
        _ => unreachable!("video backend registry and factory are out of sync"),
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
                pick_video_provider(backend, "http://localhost:3000", "test-key").is_ok(),
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
            )
            .is_err()
        );
    }

    #[test]
    fn chat_video_backend_requires_base_url() {
        assert!(pick_video_provider("openai-chat-video", "", "test-key").is_err());
    }

    #[test]
    fn chat_video_backend_requires_api_key() {
        assert!(pick_video_provider("openai-chat-video", "http://localhost:3000", "").is_err());
    }
}
