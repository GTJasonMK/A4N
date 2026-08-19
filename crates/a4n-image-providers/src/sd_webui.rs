use super::support::{
    decode_base64_image, detect_format_from_bytes, internal_error, required_non_empty,
    response_text,
};
use a4n_core::error::AppError;
use a4n_image::{ImageGenerationRequest, ImageGenerationResult, ImageProvider};
use async_trait::async_trait;
use serde_json::json;

/// Stable Diffusion WebUI provider：AUTOMATIC1111 / sd-webui API。
pub(super) struct StableDiffusionWebUiImageProvider {
    base_url: String,
    client: reqwest::Client,
}

impl StableDiffusionWebUiImageProvider {
    pub(super) fn new(base_url: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            client: reqwest::Client::new(),
        })
    }

    pub(super) fn txt2img_body(
        req: &ImageGenerationRequest,
    ) -> Result<serde_json::Value, AppError> {
        if !req.reference_images.is_empty() {
            return Err(internal_error(
                "stable-diffusion 后端使用 /sdapi/v1/txt2img，真实请求不支持参考图绑定",
            ));
        }
        let mut body = json!({
            "prompt": req.prompt,
            "width": req.width,
            "height": req.height,
            "steps": 20,
            "batch_size": 1,
            "n_iter": 1,
            "seed": -1,
            "sampler_name": "DPM++ 2M Karras",
        });

        if !req.model.trim().is_empty() {
            body["override_settings"] = json!({
                "sd_model_checkpoint": req.model,
            });
            body["override_settings_restore_afterwards"] = json!(true);
        }

        Ok(body)
    }
}

#[async_trait]
impl ImageProvider for StableDiffusionWebUiImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        let url = format!("{}/sdapi/v1/txt2img", self.base_url);
        let body = Self::txt2img_body(&req)?;

        tracing::debug!(
            "sd-webui txt2img POST {url} model={} {}x{}",
            req.model,
            req.width,
            req.height
        );

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| internal_error(format!("SD-WebUI 图像生成请求失败：{e}")))?;

        let status = resp.status();
        let text = response_text(resp, "SD-WebUI").await?;
        if !status.is_success() {
            return Err(internal_error(format!(
                "SD-WebUI 返回 {status}：{}",
                text.chars().take(500).collect::<String>()
            )));
        }

        let v: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| internal_error(format!("SD-WebUI 响应解析失败：{e}")))?;
        let b64 = v["images"][0]
            .as_str()
            .ok_or_else(|| internal_error("SD-WebUI 响应缺少 images[0]"))?;
        let bytes = decode_base64_image(b64)?;
        let format = detect_format_from_bytes(&bytes)?;
        Ok(ImageGenerationResult {
            format,
            bytes,
            model: req.model,
        })
    }
}
