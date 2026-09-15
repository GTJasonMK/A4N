use super::chat::provider::ChatCompletionsImageProvider;
use super::cliproxy::provider::CliProxyImageProvider;
use super::comfyui::ComfyUiImageProvider;
use super::openai::OpenAiCompatibleImageProvider;
use super::openai_edit::OpenAiCompatibleImageEditProvider;
use super::sd_webui::StableDiffusionWebUiImageProvider;
use super::support::internal_error;
use a4n_domain::ProviderError as AppError;
use a4n_domain::image::{
    CLIPROXY_IMAGE_BACKEND, COMFYUI_IMAGE_BACKEND, OPENAI_CHAT_IMAGE_BACKEND,
    OPENAI_COMPATIBLE_IMAGE_BACKEND, OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND,
    STABLE_DIFFUSION_IMAGE_BACKEND, image_provider_capabilities,
};
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_provider_contract::ImageProvider;
use serde_json::Value;

/// 选择 provider。
pub fn pick_image_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Box<dyn ImageProvider>, AppError> {
    if image_provider_capabilities(backend).is_none() {
        return Err(internal_error(format!("未知 backend={backend}")));
    }
    match backend {
        OPENAI_COMPATIBLE_IMAGE_BACKEND => Ok(Box::new(OpenAiCompatibleImageProvider::new(
            base_url, api_key,
        )?)),
        OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND => Ok(Box::new(
            OpenAiCompatibleImageEditProvider::new(base_url, api_key)?,
        )),
        OPENAI_CHAT_IMAGE_BACKEND => Ok(Box::new(ChatCompletionsImageProvider::new(
            base_url, api_key,
        )?)),
        CLIPROXY_IMAGE_BACKEND => Ok(Box::new(CliProxyImageProvider::new(base_url, api_key)?)),
        STABLE_DIFFUSION_IMAGE_BACKEND => {
            Ok(Box::new(StableDiffusionWebUiImageProvider::new(base_url)?))
        }
        COMFYUI_IMAGE_BACKEND => Ok(Box::new(ComfyUiImageProvider::new(base_url)?)),
        _ => unreachable!("image backend registry and factory are out of sync"),
    }
}

pub fn image_provider_request_body_preview(
    backend: &str,
    request: &ImageGenerationRequest,
) -> Result<Value, AppError> {
    match backend {
        OPENAI_COMPATIBLE_IMAGE_BACKEND => {
            OpenAiCompatibleImageProvider::request_body_preview(request)
        }
        OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND => {
            OpenAiCompatibleImageEditProvider::request_body_preview(request)
        }
        OPENAI_CHAT_IMAGE_BACKEND => {
            Ok(ChatCompletionsImageProvider::request_body_preview(request))
        }
        CLIPROXY_IMAGE_BACKEND => CliProxyImageProvider::request_body_preview(request),
        STABLE_DIFFUSION_IMAGE_BACKEND => StableDiffusionWebUiImageProvider::txt2img_body(request),
        COMFYUI_IMAGE_BACKEND => ComfyUiImageProvider::request_body_preview(request),
        other => Err(AppError::InvalidArgument(format!(
            "未知图片生成后端: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::IMAGE_PROVIDER_CAPABILITIES;

    #[test]
    fn accepts_and_previews_canonical_image_backends() {
        let request = ImageGenerationRequest {
            prompt: "draw".into(),
            width: 1024,
            height: 1024,
            model: "image-model".into(),
            reference_images: Vec::new(),
        };
        for capabilities in IMAGE_PROVIDER_CAPABILITIES {
            let backend = capabilities.backend;
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
        for capabilities in IMAGE_PROVIDER_CAPABILITIES {
            let backend = capabilities.backend;
            assert!(
                pick_image_provider(backend, "", "test-key").is_err(),
                "backend should require explicit base_url: {backend}"
            );
        }
    }

    #[test]
    fn openai_compatible_requires_api_key() {
        for capabilities in IMAGE_PROVIDER_CAPABILITIES
            .iter()
            .filter(|capabilities| capabilities.requires_api_key)
        {
            let backend = capabilities.backend;
            assert!(
                pick_image_provider(backend, "http://localhost:3000", "").is_err(),
                "backend should require api_key: {backend}"
            );
        }
        for capabilities in IMAGE_PROVIDER_CAPABILITIES
            .iter()
            .filter(|capabilities| !capabilities.requires_api_key)
        {
            assert!(pick_image_provider(capabilities.backend, "http://localhost:3000", "").is_ok());
        }
    }
}
