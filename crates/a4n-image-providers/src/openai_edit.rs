use super::openai::OpenAiCompatibleImageProvider;
use super::support::{
    build_image_http_client, image_provider_http_error, image_result_from_openai_payload,
    internal_error, prepare_reference_image_for_upload, prompt_with_reference_labels,
    reference_image_field_name, required_non_empty, response_text, send_image_request,
};
use a4n_domain::ProviderError as AppError;
use a4n_image::{ImageGenerationResult, ImageProvider};
use a4n_image_contract::ImageGenerationRequest;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use serde_json::{Value, json};

/// 支持真实图片输入的 OpenAI Images API provider。
///
/// 无参考图时走 `/images/generations`；有参考图时走 `/images/edits`，
/// 并把绑定图片作为 multipart 文件输入发送。
pub(super) struct OpenAiCompatibleImageEditProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
    generation_provider: OpenAiCompatibleImageProvider,
}

impl OpenAiCompatibleImageEditProvider {
    pub(super) fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            api_key: required_non_empty(api_key, "api_key")?.to_string(),
            client: build_image_http_client()?,
            generation_provider: OpenAiCompatibleImageProvider::new(base_url, api_key)?,
        })
    }

    fn edit_form(req: &ImageGenerationRequest) -> Result<Form, AppError> {
        if req.reference_images.is_empty() {
            return Err(internal_error("图片编辑请求缺少参考图"));
        }

        let prompt = prompt_with_reference_labels(&req.prompt, &req.reference_images);
        let mut form = Form::new()
            .text("model", req.model.clone())
            .text("prompt", prompt)
            .text("size", format!("{}x{}", req.width, req.height));
        let field_name = reference_image_field_name(req.reference_images.len());

        for (index, image) in req.reference_images.iter().enumerate() {
            let decoded = prepare_reference_image_for_upload(&image.data_uri)?;
            let filename = format!("reference_{}.{}", index + 1, decoded.format);
            let part = Part::bytes(decoded.bytes)
                .file_name(filename)
                .mime_str(&decoded.mime_type)
                .map_err(|e| internal_error(format!("参考图 multipart 构造失败：{e}")))?;
            form = form.part(field_name, part);
        }

        Ok(form)
    }

    pub(super) fn request_body_preview(req: &ImageGenerationRequest) -> Result<Value, AppError> {
        if req.reference_images.is_empty() {
            return OpenAiCompatibleImageProvider::request_body_preview(req);
        }

        let field_name = reference_image_field_name(req.reference_images.len());
        let files = req
            .reference_images
            .iter()
            .enumerate()
            .map(|(index, image)| {
                json!({
                    "field": field_name,
                    "filename": format!("reference_{}", index + 1),
                    "label": image.label,
                    "data_uri": image.data_uri,
                })
            })
            .collect::<Vec<_>>();
        let prompt = prompt_with_reference_labels(&req.prompt, &req.reference_images);

        Ok(json!({
            "endpoint": "/images/edits",
            "content_type": "multipart/form-data",
            "model": req.model,
            "size": format!("{}x{}", req.width, req.height),
            "fields": {
                "model": req.model,
                "prompt": prompt,
                "size": format!("{}x{}", req.width, req.height),
            },
            "files": files,
        }))
    }
}

#[async_trait]
impl ImageProvider for OpenAiCompatibleImageEditProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        if req.reference_images.is_empty() {
            return self.generation_provider.generate(req).await;
        }

        let url = format!("{}/images/edits", self.base_url);
        tracing::debug!(
            "image edit POST {url} model={} {}x{} refs={}",
            req.model,
            req.width,
            req.height,
            req.reference_images.len()
        );

        let form = Self::edit_form(&req)?;
        let resp = send_image_request(
            "图片编辑请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .multipart(form),
        )
        .await?;

        let status = resp.status();
        let text = response_text(resp, "图片编辑").await?;
        if !status.is_success() {
            return Err(image_provider_http_error("图片编辑", status, &text));
        }

        image_result_from_openai_payload(&self.client, &text, req.model, "图片编辑").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_image_contract::ImageReference;
    const PNG_1X1_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

    fn request(reference_count: usize) -> ImageGenerationRequest {
        ImageGenerationRequest {
            prompt: "draw a clean test image".into(),
            width: 1024,
            height: 1024,
            model: "gpt-image-2".into(),
            reference_images: (0..reference_count)
                .map(|index| ImageReference {
                    label: format!("参考图 {}", index + 1),
                    data_uri: format!("data:image/png;base64,{PNG_1X1_B64}"),
                })
                .collect(),
        }
    }

    #[test]
    fn edit_form_accepts_single_reference_image() {
        let form = OpenAiCompatibleImageEditProvider::edit_form(&request(1));
        assert!(form.is_ok());
    }

    #[test]
    fn edit_form_accepts_multiple_reference_images() {
        let form = OpenAiCompatibleImageEditProvider::edit_form(&request(2));
        assert!(form.is_ok());
    }

    #[test]
    fn edit_form_requires_reference_images() {
        let err = match OpenAiCompatibleImageEditProvider::edit_form(&request(0)) {
            Ok(_) => panic!("edit form should require reference images"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("缺少参考图"));
    }
}
