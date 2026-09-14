use super::super::support::{
    build_image_http_client, image_provider_http_error, image_result_from_openai_payload,
    required_non_empty, response_text, send_image_request,
};
use a4n_domain::ProviderError as AppError;
use a4n_image::{ImageGenerationResult, ImageProvider};
use a4n_image_contract::ImageGenerationRequest;
use async_trait::async_trait;

/// CLIProxyAPI image provider for xAI Grok Imagine Image.
pub(crate) struct CliProxyImageProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl CliProxyImageProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            api_key: required_non_empty(api_key, "api_key")?.to_string(),
            client: build_image_http_client()?,
        })
    }
}
#[async_trait]
impl ImageProvider for CliProxyImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        if req.reference_images.is_empty() {
            let url = format!("{}/images/generations", self.base_url);
            let body = Self::generation_body(&req);
            tracing::debug!(
                "cliproxy image generate POST {url} model={} {}x{}",
                req.model,
                req.width,
                req.height
            );
            let resp = send_image_request(
                "CLIProxy 图片生成请求失败",
                self.client
                    .post(&url)
                    .bearer_auth(&self.api_key)
                    .json(&body),
            )
            .await?;
            let status = resp.status();
            let text = response_text(resp, "CLIProxy 图片生成").await?;
            if !status.is_success() {
                return Err(image_provider_http_error(
                    "CLIProxy 图片生成",
                    status,
                    &text,
                ));
            }
            return image_result_from_openai_payload(
                &self.client,
                &text,
                req.model,
                "CLIProxy 图片生成",
            )
            .await;
        }

        let url = format!("{}/images/edits", self.base_url);
        tracing::debug!(
            "cliproxy image edit POST {url} model={} {}x{} refs={}",
            req.model,
            req.width,
            req.height,
            req.reference_images.len()
        );
        let form = self.edit_form(&req)?;
        let resp = send_image_request(
            "CLIProxy 图片编辑请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .multipart(form),
        )
        .await?;
        let status = resp.status();
        let text = response_text(resp, "CLIProxy 图片编辑").await?;
        if !status.is_success() {
            return Err(image_provider_http_error(
                "CLIProxy 图片编辑",
                status,
                &text,
            ));
        }
        image_result_from_openai_payload(&self.client, &text, req.model, "CLIProxy 图片编辑").await
    }
}
