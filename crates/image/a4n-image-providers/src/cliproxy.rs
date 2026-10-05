mod request;
#[cfg(test)]
mod tests;

use crate::support::{
    build_image_http_client, image_provider_http_error, image_result_from_openai_payload,
    required_non_empty, response_text, send_image_request,
};
use a4n_domain::ProviderError as AppError;
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_provider_contract::{ImageGenerationResult, ImageProvider};
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
        let body = Self::edit_body(&req)?;
        let resp = send_image_request(
            "CLIProxy 图片编辑请求失败",
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&body),
        )
        .await?;
        let status = resp.status();
        let text = response_text(resp, "CLIProxy 图片编辑").await?;
        if !status.is_success() {
            return Err(cli_proxy_image_edit_error(&req.model, status, &text));
        }
        image_result_from_openai_payload(&self.client, &text, req.model, "CLIProxy 图片编辑").await
    }
}

fn cli_proxy_image_edit_error(model: &str, status: reqwest::StatusCode, body: &str) -> AppError {
    let normalized_body = body.to_ascii_lowercase();
    if normalized_body.contains("personal-team-blocked:spending-limit") {
        return AppError::InvalidArgument(format!(
            "CLIProxy/Grok 图片编辑被 XAI API/team 消费策略拒绝：CPA 返回 personal-team-blocked:spending-limit。网页每周 SuperGrok 额度与 API billing 是两套限制；请在 XAI/CPA billing 中启用图片 API 额度或更换有 API 权限的账号（当前模型：{model}）。"
        ));
    }

    if status == reqwest::StatusCode::FORBIDDEN
        && normalized_body.contains("model does not support this api endpoint")
    {
        return AppError::InvalidArgument(format!(
            "CLIProxy/Grok `/images/edits` 请求返回 403：Model does not support this API endpoint。该错误可能由 CPAproxy/Gateway 的模型目录在转发前生成；Grok xAI 参考图请求使用 JSON image.url/images[].url 合同。请检查当前模型 {model} 的 endpoint_families 是否包含 images_xai。无参考图生成可继续使用 /images/generations。"
        ));
    }

    image_provider_http_error("CLIProxy 图片编辑", status, body)
}
