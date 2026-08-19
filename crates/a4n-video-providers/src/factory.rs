use a4n_core::error::AppError;

use super::chat::ChatCompletionsVideoProvider;
use super::cliproxy_openai::provider::CliProxyOpenAiVideoProvider;
use super::gemini_veo::provider::GeminiVeoVideoProvider;
use super::http::provider::HttpVideoProvider;
use super::newapi::provider::NewApiVideoProvider;
use super::seedance2::provider::Seedance2VideoProvider;
use a4n_video::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
    VideoGenerationRequest, VideoProvider, video_provider_capabilities,
};
use serde_json::Value;

pub fn pick_video_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Box<dyn VideoProvider>, AppError> {
    if video_provider_capabilities(backend).is_none() {
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

pub fn video_provider_request_body_preview(
    backend: &str,
    base_url: &str,
    request: &VideoGenerationRequest,
) -> Result<Value, AppError> {
    match backend {
        OPENAI_COMPATIBLE_VIDEO_BACKEND => Ok(HttpVideoProvider::request_body(request)),
        NEWAPI_VIDEO_BACKEND => NewApiVideoProvider::request_body(request),
        GEMINI_VEO_VIDEO_BACKEND => GeminiVeoVideoProvider::request_body_preview(request),
        SEEDANCE2_VIDEO_BACKEND => Seedance2VideoProvider::request_body(request),
        CLIPROXY_OPENAI_VIDEO_BACKEND => {
            CliProxyOpenAiVideoProvider::request_body_preview_for_base_url(request, base_url)
        }
        OPENAI_CHAT_VIDEO_BACKEND => {
            Ok(ChatCompletionsVideoProvider::request_body_preview(request))
        }
        other => Err(AppError::InvalidArgument(format!(
            "未知视频生成后端: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_video::{VIDEO_PROVIDER_CAPABILITIES, VideoMediaInput};

    #[test]
    fn accepts_canonical_video_backends() {
        let request = VideoGenerationRequest {
            prompt: "test".into(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 5,
            fps: 24,
            model: "video-model".into(),
            media: VideoMediaInput::None,
        };
        for backend in VIDEO_PROVIDER_CAPABILITIES {
            let mut preview_request = request.clone();
            if backend.backend == SEEDANCE2_VIDEO_BACKEND {
                preview_request.media = VideoMediaInput::References {
                    images: vec!["data:image/png;base64,AA==".into()],
                    videos: Vec::new(),
                    audio: Vec::new(),
                };
            }
            assert!(
                pick_video_provider(backend.backend, "http://localhost:3000", "test-key").is_ok(),
                "backend should be accepted: {}",
                backend.backend
            );
            assert!(
                video_provider_request_body_preview(
                    backend.backend,
                    "http://localhost:3000/v1",
                    &preview_request,
                )
                .is_ok(),
                "backend request preview should be available: {}",
                backend.backend
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
        assert!(
            video_provider_request_body_preview(
                "unsupported-video-backend",
                "http://localhost:3000",
                &VideoGenerationRequest {
                    prompt: "test".into(),
                    negative_prompt: None,
                    width: 1280,
                    height: 720,
                    duration_secs: 5,
                    fps: 24,
                    model: "video-model".into(),
                    media: VideoMediaInput::None,
                },
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
