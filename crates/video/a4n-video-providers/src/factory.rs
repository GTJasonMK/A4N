use a4n_domain::ProviderError as AppError;

use super::chat::ChatCompletionsVideoProvider;
use super::cliproxy_openai::CliProxyOpenAiVideoProvider;
use super::gemini_veo::GeminiVeoVideoProvider;
use super::http::HttpVideoProvider;
use super::newapi::NewApiVideoProvider;
use super::seedance2::Seedance2VideoProvider;
use a4n_video_provider_contract::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
    VideoProvider,
};
use a4n_video_sidecar_contract::{
    VideoGenerationInput, VideoMediaSource, validate_video_image_artifact,
};
use serde_json::Value;

pub fn pick_video_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Box<dyn VideoProvider>, AppError> {
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
        _ => Err(AppError::Internal(format!("未知视频 backend={backend}"))),
    }
}

pub fn video_provider_request_body_preview(
    backend: &str,
    base_url: &str,
    request: &VideoGenerationInput,
) -> Result<Value, AppError> {
    request
        .media
        .validate_artifacts()
        .map_err(AppError::InvalidArgument)?;
    let request = request
        .clone()
        .try_map_images(|source| -> Result<String, AppError> {
            match source {
                VideoMediaSource::Url { url } => Ok(url),
                VideoMediaSource::Artifact { artifact } => {
                    validate_video_image_artifact(&artifact).map_err(AppError::InvalidArgument)?;
                    Ok(format!(
                        "https://example.invalid/a4n-media/preview/{}",
                        artifact.artifact_id
                    ))
                }
            }
        })?;
    let request = &request;
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
    use a4n_video_provider_contract::VIDEO_PROVIDER_CAPABILITIES;
    use a4n_video_sidecar_contract::{
        VideoGenerationInput as VideoGenerationRequest, VideoMediaInput,
    };

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
