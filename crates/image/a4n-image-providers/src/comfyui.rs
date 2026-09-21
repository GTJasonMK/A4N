use super::support::{
    detect_format_from_bytes, internal_error, percent_encode_query, required_json_string,
    required_json_string_field, required_non_empty, response_bytes, response_text, time_seed,
};
use a4n_domain::ProviderError as AppError;
use a4n_image_contract::ImageGenerationRequest;
use a4n_image_provider_contract::{ImageGenerationResult, ImageProvider};
use async_trait::async_trait;
use serde_json::json;
use std::time::Duration;

/// ComfyUI provider：使用基础 CheckpointLoaderSimple 文生图 workflow。
pub(super) struct ComfyUiImageProvider {
    base_url: String,
    client: reqwest::Client,
}

impl ComfyUiImageProvider {
    pub(super) fn new(base_url: &str) -> Result<Self, AppError> {
        Ok(Self {
            base_url: required_non_empty(base_url, "base_url")?
                .trim_end_matches('/')
                .to_string(),
            client: reqwest::Client::new(),
        })
    }

    fn workflow(req: &ImageGenerationRequest) -> Result<serde_json::Value, AppError> {
        Ok(json!({
            "3": {
                "class_type": "KSampler",
                "inputs": {
                    "seed": time_seed()?,
                    "steps": 24,
                    "cfg": 7.0,
                    "sampler_name": "euler",
                    "scheduler": "normal",
                    "denoise": 1.0,
                    "model": ["4", 0],
                    "positive": ["6", 0],
                    "negative": ["7", 0],
                    "latent_image": ["5", 0]
                }
            },
            "4": {
                "class_type": "CheckpointLoaderSimple",
                "inputs": {
                    "ckpt_name": req.model
                }
            },
            "5": {
                "class_type": "EmptyLatentImage",
                "inputs": {
                    "width": req.width,
                    "height": req.height,
                    "batch_size": 1
                }
            },
            "6": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": req.prompt,
                    "clip": ["4", 1]
                }
            },
            "7": {
                "class_type": "CLIPTextEncode",
                "inputs": {
                    "text": "",
                    "clip": ["4", 1]
                }
            },
            "8": {
                "class_type": "VAEDecode",
                "inputs": {
                    "samples": ["3", 0],
                    "vae": ["4", 2]
                }
            },
            "9": {
                "class_type": "SaveImage",
                "inputs": {
                    "filename_prefix": "a4n",
                    "images": ["8", 0]
                }
            }
        }))
    }

    fn submit_body(
        req: &ImageGenerationRequest,
        client_id: String,
    ) -> Result<serde_json::Value, AppError> {
        if !req.reference_images.is_empty() {
            return Err(internal_error(
                "comfyui 后端使用文生图 workflow，真实请求不支持参考图绑定",
            ));
        }
        if req.model.trim().is_empty() {
            return Err(internal_error("ComfyUI 配置需要填写 ckpt_name 模型文件名"));
        }
        Ok(json!({
            "client_id": client_id,
            "prompt": Self::workflow(req)?,
        }))
    }

    pub(super) fn request_body_preview(
        req: &ImageGenerationRequest,
    ) -> Result<serde_json::Value, AppError> {
        Self::submit_body(req, "a4n-preview".to_string())
    }
}

#[async_trait]
impl ImageProvider for ComfyUiImageProvider {
    async fn generate(
        &self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResult, AppError> {
        let client_id = format!("a4n-{}", time_seed()?);
        let submit_url = format!("{}/prompt", self.base_url);
        let submit_body = Self::submit_body(&req, client_id)?;

        tracing::debug!(
            "comfyui prompt POST {submit_url} model={} {}x{}",
            req.model,
            req.width,
            req.height
        );

        let resp = self
            .client
            .post(&submit_url)
            .json(&submit_body)
            .send()
            .await
            .map_err(|e| internal_error(format!("ComfyUI 提交任务失败：{e}")))?;
        let status = resp.status();
        let text = response_text(resp, "ComfyUI 提交任务").await?;
        if !status.is_success() {
            return Err(internal_error(format!(
                "ComfyUI 提交任务返回 {status}：{}",
                text.chars().take(500).collect::<String>()
            )));
        }
        let submitted: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| internal_error(format!("ComfyUI 提交响应解析失败：{e}")))?;
        let prompt_id = required_json_string(&submitted["prompt_id"], "prompt_id")?;

        let history_url = format!(
            "{}/history/{}",
            self.base_url,
            percent_encode_query(prompt_id)
        );
        let mut history: Option<serde_json::Value> = None;
        for _ in 0..120 {
            let resp = self
                .client
                .get(&history_url)
                .send()
                .await
                .map_err(|e| internal_error(format!("ComfyUI 查询 history 失败：{e}")))?;
            let status = resp.status();
            if !status.is_success() {
                let text = response_text(resp, "ComfyUI history").await?;
                return Err(internal_error(format!(
                    "ComfyUI history 返回 {status}：{}",
                    text.chars().take(500).collect::<String>()
                )));
            }
            let value: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| internal_error(format!("ComfyUI history 解析失败：{e}")))?;
            if value
                .get(prompt_id)
                .and_then(|v| v.get("outputs"))
                .is_some()
            {
                history = Some(value);
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        let history = history.ok_or_else(|| internal_error("ComfyUI 图像生成超时"))?;
        let outputs = history
            .get(prompt_id)
            .and_then(|v| v.get("outputs"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| internal_error("ComfyUI history 缺少 outputs"))?;

        let image = outputs
            .values()
            .find_map(|node| {
                node.get("images")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
            })
            .ok_or_else(|| internal_error("ComfyUI 未返回输出图片"))?;
        let filename = required_json_string(&image["filename"], "filename")?;
        let subfolder = required_json_string_field(&image["subfolder"], "subfolder")?;
        let image_type = required_json_string(&image["type"], "type")?;
        let view_url = format!(
            "{}/view?filename={}&subfolder={}&type={}",
            self.base_url,
            percent_encode_query(filename),
            percent_encode_query(subfolder),
            percent_encode_query(image_type)
        );
        let bytes = self
            .client
            .get(&view_url)
            .send()
            .await
            .map_err(|e| internal_error(format!("ComfyUI 下载图片失败：{e}")))?;
        let bytes = response_bytes(bytes, "ComfyUI 下载图片").await?;
        let format = detect_format_from_bytes(&bytes)?;

        Ok(ImageGenerationResult {
            format,
            bytes,
            model: req.model,
        })
    }
}
