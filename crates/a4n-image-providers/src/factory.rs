use super::chat::ChatCompletionsImageProvider;
use super::cliproxy::CliProxyImageProvider;
use super::comfyui::ComfyUiImageProvider;
use super::openai::OpenAiCompatibleImageProvider;
use super::openai_edit::OpenAiCompatibleImageEditProvider;
use super::sd_webui::StableDiffusionWebUiImageProvider;
use super::support::internal_error;
use a4n_core::error::AppError;
use a4n_image::{ImageGenerationRequest, ImageProvider};
use serde_json::Value;

/// 选择 provider。
pub fn pick_image_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Box<dyn ImageProvider>, AppError> {
    match backend {
        "openai-compatible-image" => Ok(Box::new(OpenAiCompatibleImageProvider::new(
            base_url, api_key,
        )?)),
        "openai-compatible-image-edit" => Ok(Box::new(OpenAiCompatibleImageEditProvider::new(
            base_url, api_key,
        )?)),
        "openai-chat-image" => Ok(Box::new(ChatCompletionsImageProvider::new(
            base_url, api_key,
        )?)),
        "cliproxy-image" => Ok(Box::new(CliProxyImageProvider::new(base_url, api_key)?)),
        "stable-diffusion" => Ok(Box::new(StableDiffusionWebUiImageProvider::new(base_url)?)),
        "comfyui" => Ok(Box::new(ComfyUiImageProvider::new(base_url)?)),
        other => Err(internal_error(format!("未知 backend={other}"))),
    }
}

pub fn image_provider_request_body_preview(
    backend: &str,
    request: &ImageGenerationRequest,
) -> Result<Value, AppError> {
    match backend {
        "openai-compatible-image" => OpenAiCompatibleImageProvider::request_body_preview(request),
        "openai-compatible-image-edit" => {
            OpenAiCompatibleImageEditProvider::request_body_preview(request)
        }
        "openai-chat-image" => Ok(ChatCompletionsImageProvider::request_body_preview(request)),
        "cliproxy-image" => CliProxyImageProvider::request_body_preview(request),
        "stable-diffusion" => StableDiffusionWebUiImageProvider::request_body_preview(request),
        "comfyui" => ComfyUiImageProvider::request_body_preview(request),
        other => Err(AppError::InvalidArgument(format!(
            "未知图片生成后端: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_and_previews_canonical_image_backends() {
        let request = ImageGenerationRequest {
            prompt: "draw".into(),
            width: 1024,
            height: 1024,
            model: "image-model".into(),
            reference_images: Vec::new(),
        };
        for backend in [
            "openai-compatible-image",
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
            "stable-diffusion",
            "comfyui",
        ] {
            assert!(
                pick_image_provider(backend, "http://localhost:3000", "test-key").is_ok(),
                "backend should be accepted: {backend}"
            );
            assert!(
                image_provider_request_body_preview(backend, &request).is_ok(),
                "backend request preview should be available: {backend}"
            );
        }
    }

    #[test]
    fn rejects_unknown_image_backend() {
        assert!(
            pick_image_provider(
                "unsupported-image-backend",
                "http://localhost:3000",
                "test-key",
            )
            .is_err()
        );
    }

    #[test]
    fn rejects_empty_provider_base_url() {
        for backend in [
            "openai-compatible-image",
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
            "stable-diffusion",
            "comfyui",
        ] {
            assert!(
                pick_image_provider(backend, "", "test-key").is_err(),
                "backend should require explicit base_url: {backend}"
            );
        }
    }

    #[test]
    fn openai_compatible_requires_api_key() {
        for backend in [
            "openai-compatible-image",
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
        ] {
            assert!(
                pick_image_provider(backend, "http://localhost:3000", "").is_err(),
                "backend should require api_key: {backend}"
            );
        }
        assert!(pick_image_provider("stable-diffusion", "http://localhost:3000", "").is_ok());
        assert!(pick_image_provider("comfyui", "http://localhost:3000", "").is_ok());
    }
}
