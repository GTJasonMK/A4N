use super::chat::ChatCompletionsImageProvider;
use super::cliproxy::CliProxyImageProvider;
use super::comfyui::ComfyUiImageProvider;
use super::openai::OpenAiCompatibleImageProvider;
use super::openai_edit::OpenAiCompatibleImageEditProvider;
use super::sd_webui::StableDiffusionWebUiImageProvider;
use super::support::internal_error;
use a4n_core::error::AppError;
use a4n_image::ImageProvider;
use serde_json::Value;

/// 选择 provider。
pub fn pick_image_provider(
    backend: &str,
    base_url: &str,
    api_key: &str,
    provider_options_json: Option<&Value>,
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
        "cliproxy-image" => Ok(Box::new(CliProxyImageProvider::new_with_options(
            base_url,
            api_key,
            provider_options_json,
        )?)),
        "stable-diffusion" => Ok(Box::new(StableDiffusionWebUiImageProvider::new(base_url)?)),
        "comfyui" => Ok(Box::new(ComfyUiImageProvider::new(base_url)?)),
        other => Err(internal_error(format!("未知 backend={other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_image_backends() {
        for backend in [
            "openai-compatible-image",
            "openai-compatible-image-edit",
            "openai-chat-image",
            "cliproxy-image",
            "stable-diffusion",
            "comfyui",
        ] {
            assert!(
                pick_image_provider(backend, "http://localhost:3000", "test-key", None).is_ok(),
                "backend should be accepted: {backend}"
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
                None,
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
                pick_image_provider(backend, "", "test-key", None).is_err(),
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
                pick_image_provider(backend, "http://localhost:3000", "", None).is_err(),
                "backend should require api_key: {backend}"
            );
        }
        assert!(pick_image_provider("stable-diffusion", "http://localhost:3000", "", None).is_ok());
        assert!(pick_image_provider("comfyui", "http://localhost:3000", "", None).is_ok());
    }
}
