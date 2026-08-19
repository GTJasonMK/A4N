use super::support::{
    build_image_http_client, image_provider_http_error, image_result_from_openai_payload,
    internal_error, required_non_empty, response_text, send_image_request,
};
use a4n_core::error::AppError;
use a4n_image::{ImageGenerationRequest, ImageGenerationResult, ImageProvider};
use async_trait::async_trait;
use serde_json::json;

/// OpenAI-compatible image provider.
pub(super) struct OpenAiCompatibleImageProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl OpenAiCompatibleImageProvider {
    pub(super) fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            api_key: required_non_empty(api_key, "api_key")?.to_string(),
            client: build_image_http_client()?,
        })
    }

    fn generation_body(req: &ImageGenerationRequest) -> serde_json::Value {
        json!({
            "model": req.model,
            "prompt": req.prompt,
            "n": 1,
            "size": format!("{}x{}", req.width, req.height),
        })
    }

    fn validate_generation_request(req: &ImageGenerationRequest) -> Result<(), AppError> {
        if !req.reference_images.is_empty() {
            return Err(internal_error(
                "当前 openai-compatible-image 后端只支持 /images/generations 文字生图；参考图输入请改用 openai-compatible-image-edit、openai-chat-image 或 cliproxy-image",
            ));
        }
        Ok(())
    }

    pub(super) fn request_body_preview(
        req: &ImageGenerationRequest,
    ) -> Result<serde_json::Value, AppError> {
        Self::validate_generation_request(req)?;
        Ok(Self::generation_body(req))
    }
}

#[async_trait]
impl ImageProvider for OpenAiCompatibleImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        Self::validate_generation_request(&req)?;
        let url = format!("{}/images/generations", self.base_url);
        let body = Self::generation_body(&req);

        tracing::debug!(
            "image generate POST {url} model={} {}x{}",
            req.model,
            req.width,
            req.height
        );

        let resp = send_image_request(
            "图像生成请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body),
        )
        .await?;

        let status = resp.status();
        let text = response_text(resp, "图像生成").await?;
        if !status.is_success() {
            return Err(image_provider_http_error("图像生成", status, &text));
        }

        image_result_from_openai_payload(&self.client, &text, req.model, "图像生成").await
    }
}

#[cfg(test)]
mod tests {
    use super::super::support::{
        decode_base64_image, image_format_hint, optional_format_from_response_item,
    };
    use super::*;
    use base64::Engine as _;

    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    fn request() -> ImageGenerationRequest {
        ImageGenerationRequest {
            prompt: "draw a clean test image".into(),
            width: 1024,
            height: 1024,
            model: "gpt-image-2".into(),
            reference_images: Vec::new(),
        }
    }

    #[test]
    fn generation_body_does_not_force_large_b64_response() {
        let body = OpenAiCompatibleImageProvider::generation_body(&request());

        assert_eq!(body["model"], "gpt-image-2");
        assert_eq!(body["size"], "1024x1024");
        assert!(body.get("response_format").is_none());
    }

    #[test]
    fn openai_compatible_b64_result_does_not_require_format_field() {
        let item = json!({ "b64_json": PNG_1X1_B64 });
        let declared_format = optional_format_from_response_item(&item).unwrap();
        let bytes = decode_base64_image(item["b64_json"].as_str().unwrap()).unwrap();

        let format = image_format_hint(declared_format.as_deref(), None, None, &bytes).unwrap();

        assert_eq!(format, "png");
    }

    #[test]
    fn openai_compatible_url_result_can_infer_format_from_url() {
        let item = json!({ "url": "https://example.com/generated.jpeg" });
        let declared_format = optional_format_from_response_item(&item).unwrap();

        let format =
            image_format_hint(declared_format.as_deref(), item["url"].as_str(), None, &[]).unwrap();

        assert_eq!(format, "jpg");
    }

    #[test]
    fn openai_compatible_prefers_declared_mime_type() {
        let item = json!({
            "b64_json": base64::engine::general_purpose::STANDARD.encode(b"opaque"),
            "mime_type": "image/webp"
        });
        let declared_format = optional_format_from_response_item(&item).unwrap();

        assert_eq!(declared_format.as_deref(), Some("webp"));
    }
}
