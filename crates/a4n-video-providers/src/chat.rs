mod payload;
mod request;
mod response_parser;

#[cfg(test)]
mod tests;

use async_trait::async_trait;

use a4n_core::error::AppError;

use payload::ChatVideoPayload;
use request::chat_request_body;
use response_parser::extract_chat_video_payload;

use super::download::download_video_url;
use super::format::detect_video_format_from_bytes;
use super::transport::{self, ApiKeyAuth};
use a4n_video::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

/// Chat-Completions 视频 provider：把 `/v1/chat/completions` 当作视频生成网关。
///
/// **响应解析策略**：按明确支持的 Chat-Completions 视频响应结构查找视频引用：
///
/// 1. `choices[0].message.video` 对象 / `choices[0].message.videos[*]` 数组
/// 2. `choices[0].message.content[*]` 数组中 `type` 为 `video_url` / `video` /
///    `output_video` / `input_video` 的项
/// 3. `choices[0].message.content` 字符串里的 `data:video/*;base64,…`、markdown
///    视频 `![alt](url)`、或以 mp4/webm/mov/m4v 结尾的裸 URL
///
/// chat completions 协议**只有同步返回**，`generate()` 直接拉回字节；`poll()` 永远报错。
pub struct ChatCompletionsVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl ChatCompletionsVideoProvider {
    pub fn new(base_url: &str, api_key: &str) -> Result<Self, AppError> {
        let base = base_url.trim();
        if base.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 base_url".into()));
        }
        let key = api_key.trim();
        if key.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 api_key".into()));
        }
        Ok(Self {
            base_url: base.trim_end_matches('/').to_string(),
            api_key: key.to_string(),
            client: reqwest::Client::new(),
        })
    }

    #[cfg(test)]
    fn parse_response_for_test(body: &str) -> Result<ChatVideoPayload, AppError> {
        extract_chat_video_payload(body)
    }

    pub fn request_body_preview(req: &VideoGenerationRequest) -> serde_json::Value {
        let user_content = video_user_content(req);
        chat_request_body(req, user_content)
    }
}

#[async_trait]
impl VideoProvider for ChatCompletionsVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<VideoGenerationResult, AppError> {
        let url = format!("{}/chat/completions", self.base_url);
        let user_content = video_user_content(&req);
        let body = chat_request_body(&req, user_content);

        tracing::debug!(
            "chat-completions video POST {url} model={} {}x{} {}s {}fps",
            req.model,
            req.width,
            req.height,
            req.duration_secs,
            req.fps
        );

        let raw_value = transport::post_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::Bearer,
            &url,
            &body,
            provider_idempotency_key_sha256,
            "chat-completions 视频生成",
        )
        .await?;
        let raw_text = serde_json::to_string(&raw_value).map_err(|e| {
            AppError::Internal(anyhow::anyhow!(
                "chat-completions 视频响应重新序列化失败：{e}"
            ))
        })?;
        let payload = extract_chat_video_payload(&raw_text)?;

        let (bytes, format) = match payload {
            ChatVideoPayload::DataUri { format, bytes } => (bytes, format),
            ChatVideoPayload::Base64 { format, bytes } => {
                let format = match format {
                    Some(f) => f,
                    None => detect_video_format_from_bytes(&bytes)?,
                };
                (bytes, format)
            }
            ChatVideoPayload::Url { url, hint_format } => {
                tracing::debug!("chat-completions 返回 URL，下载：{url}");
                let (bytes, format) =
                    download_video_url(&self.client, &url, hint_format.as_deref()).await?;
                (bytes, format)
            }
        };

        Ok(VideoGenerationResult {
            bytes,
            format,
            // chat completions 协议同步返回，没有可供轮询的 job id。
            provider_job_id: None,
            metadata: raw_value,
        })
    }

    async fn poll(&self, _job_id: &str) -> Result<VideoJobState, AppError> {
        Err(AppError::Internal(anyhow::anyhow!(
            "chat-completions 视频 backend 仅支持同步返回，不支持轮询（若需异步任务请改用 openai-compatible-video）"
        )))
    }

    async fn status(&self, _job_id: &str) -> Result<a4n_video::VideoJobStatus, AppError> {
        Err(AppError::InvalidArgument(
            "chat-completions 视频 backend 仅支持同步返回，不支持状态查询".to_string(),
        ))
    }
}

fn video_user_content(req: &VideoGenerationRequest) -> String {
    format!(
        "{}\n\n[输出视频，尺寸 {}x{}，时长 {}s，帧率 {}fps]",
        req.prompt, req.width, req.height, req.duration_secs, req.fps
    )
}
