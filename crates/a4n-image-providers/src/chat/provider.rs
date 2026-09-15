use super::parser::{ChatImagePayload, extract_chat_image_payload};
use super::request::chat_request_body;
use crate::support::{
    build_image_http_client, detect_format_from_bytes, download_image_url,
    image_provider_http_error, required_non_empty, response_text, send_image_request,
};
use a4n_domain::ProviderError as AppError;
use a4n_image::{ImageGenerationResult, ImageProvider};
use a4n_image_contract::ImageGenerationRequest;
use async_trait::async_trait;
use serde_json::Value;

pub(crate) struct ChatCompletionsImageProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl ChatCompletionsImageProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            api_key: required_non_empty(api_key, "api_key")?.to_string(),
            client: build_image_http_client()?,
        })
    }

    #[cfg(test)]
    pub(super) fn parse_response_for_test(body: &str) -> Result<ChatImagePayload, AppError> {
        extract_chat_image_payload(body)
    }

    pub(crate) fn request_body_preview(req: &ImageGenerationRequest) -> Value {
        chat_request_body(req)
    }
}

#[async_trait]
impl ImageProvider for ChatCompletionsImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = chat_request_body(&req);

        tracing::debug!(
            "chat-completions image POST {url} model={} {}x{}",
            req.model,
            req.width,
            req.height
        );

        let resp = send_image_request(
            "chat-completions 图像生成请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .header(reqwest::header::ACCEPT, "application/json")
                .json(&body),
        )
        .await?;

        let status = resp.status();
        let text = response_text(resp, "chat-completions 图像生成").await?;
        if !status.is_success() {
            return Err(image_provider_http_error(
                "chat-completions 图像生成",
                status,
                &text,
            ));
        }

        let payload = extract_chat_image_payload(&text)?;
        let (bytes, format) = match payload {
            ChatImagePayload::DataUri { format, bytes } => (bytes, format),
            ChatImagePayload::Base64 { format, bytes } => {
                let format = match format {
                    Some(f) => f,
                    None => detect_format_from_bytes(&bytes)?,
                };
                (bytes, format)
            }
            ChatImagePayload::Url(url) => {
                tracing::debug!("chat-completions 返回 URL，下载：{url}");
                download_image_url(&self.client, &url, None).await?
            }
        };

        Ok(ImageGenerationResult {
            bytes,
            format,
            model: req.model,
        })
    }
}
