use super::request::{grok_reference_mode, request_body_with_mode};
use super::response::{
    completed_video_url, error_body_preview, failure_message, format_from_content_type,
    is_completed_status, status, url_escape_path_segment, video_id,
};
use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use a4n_core::error::AppError;
use a4n_video::{
    VideoGenerationRequest, VideoGenerationResult, VideoJobDeleteOutcome, VideoJobState,
    VideoProvider,
};

use crate::download::download_video_url;
use crate::format::detect_video_format_from_bytes;
use crate::response::{parse_json_response, parse_json_response_allow_not_found};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GrokReferenceMode {
    Reference,
    FirstFrame,
}

/// CLIProxyAPI `/openai/v1/videos` adapter for xAI Grok Imagine Video.
pub struct CliProxyOpenAiVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
    reference_mode: GrokReferenceMode,
}

impl CliProxyOpenAiVideoProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
            reference_mode: GrokReferenceMode::Reference,
        }
    }

    pub fn new_with_options(
        base_url: &str,
        api_key: &str,
        provider_options_json: Option<&Value>,
    ) -> Result<Self, AppError> {
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
            reference_mode: grok_reference_mode(provider_options_json)?,
        })
    }

    fn videos_url(&self) -> String {
        if self.base_url.ends_with("/videos") {
            self.base_url.clone()
        } else {
            format!("{}/videos", self.base_url)
        }
    }

    fn video_url(&self, video_id: &str) -> String {
        format!(
            "{}/{}",
            self.videos_url(),
            url_escape_path_segment(video_id)
        )
    }

    pub(super) fn content_url(&self, video_id: &str) -> String {
        format!("{}/content?variant=video", self.video_url(video_id))
    }

    pub(super) fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        request_body_with_mode(req, GrokReferenceMode::Reference)
    }

    fn request_body_for_config(&self, req: &VideoGenerationRequest) -> Result<Value, AppError> {
        request_body_with_mode(req, self.reference_mode)
    }

    pub fn request_body_preview(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        Self::request_body(req)
    }

    pub fn request_body_preview_with_options(
        req: &VideoGenerationRequest,
        provider_options_json: Option<&Value>,
    ) -> Result<Value, AppError> {
        request_body_with_mode(req, grok_reference_mode(provider_options_json)?)
    }

    async fn post_json(
        &self,
        url: &str,
        body: Value,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut request = self.client.post(url).json(&body);
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
        let request = crate::idempotency::with_provider_idempotency_key(
            request,
            provider_idempotency_key_sha256,
        )?;
        let resp = request
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("CLIProxy 视频生成请求失败：{e}")))?;
        parse_json_response(resp, "CLIProxy 视频生成").await
    }

    async fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let mut request = self.client.get(url);
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
        let resp = request
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("CLIProxy 视频任务查询失败：{e}")))?;
        parse_json_response(resp, "CLIProxy 视频任务查询").await
    }

    async fn get_status_json(&self, url: &str) -> Result<Option<Value>, AppError> {
        let mut request = self.client.get(url);
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
        let resp = request.send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("CLIProxy 视频任务状态查询失败：{e}"))
        })?;
        parse_json_response_allow_not_found(resp, "CLIProxy 视频任务状态查询").await
    }

    async fn download_completed_video(
        &self,
        video_id: &str,
        metadata: Value,
    ) -> Result<VideoGenerationResult, AppError> {
        if let Some(video_url) = completed_video_url(&metadata) {
            let (bytes, format) = download_video_url(&self.client, video_url, None).await?;
            return Ok(VideoGenerationResult {
                bytes,
                format,
                provider_job_id: Some(video_id.to_string()),
                metadata,
            });
        }

        if self.supports_openai_content_route() {
            return self.download_openai_content(video_id, metadata).await;
        }

        Err(AppError::Internal(anyhow::anyhow!(
            "CLIProxy native 视频任务已完成，但响应缺少 video.url，无法下载；请确认 GET /v1/videos/{video_id} 返回 video.url"
        )))
    }

    pub(super) fn supports_openai_content_route(&self) -> bool {
        self.videos_url().contains("/openai/")
    }

    async fn download_openai_content(
        &self,
        video_id: &str,
        metadata: Value,
    ) -> Result<VideoGenerationResult, AppError> {
        let url = self.content_url(video_id);
        let mut request = self.client.get(&url);
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
        let resp = request
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("CLIProxy 视频下载请求失败：{e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.map_err(|e| {
                AppError::Internal(anyhow::anyhow!("读取 CLIProxy 视频下载错误响应失败：{e}"))
            })?;
            return Err(AppError::Internal(anyhow::anyhow!(
                "CLIProxy 视频下载返回 {status}：{}",
                error_body_preview(&text)
            )));
        }
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("读取 CLIProxy 视频字节失败：{e}")))?
            .to_vec();
        let format = content_type
            .as_deref()
            .and_then(format_from_content_type)
            .or_else(|| detect_video_format_from_bytes(&bytes).ok())
            .unwrap_or_else(|| "mp4".to_string());
        Ok(VideoGenerationResult {
            bytes,
            format,
            provider_job_id: Some(video_id.to_string()),
            metadata,
        })
    }
}

#[async_trait]
impl VideoProvider for CliProxyOpenAiVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.videos_url();
        let body = self.request_body_for_config(&req)?;
        tracing::debug!("cliproxy video POST {url} model={}", req.model);
        let v = self
            .post_json(&url, body, provider_idempotency_key_sha256)
            .await?;
        let video_id = video_id(&v)?;

        if is_completed_status(status(&v).as_deref()) {
            return self.download_completed_video(&video_id, v).await;
        }
        if let Some(message) = failure_message(&v) {
            return Err(AppError::Internal(anyhow::anyhow!(
                "CLIProxy 视频生成失败：{message}"
            )));
        }

        Ok(VideoGenerationResult {
            bytes: Vec::new(),
            format: String::new(),
            provider_job_id: Some(video_id),
            metadata: v,
        })
    }

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let video_id = job_id.trim();
        if video_id.is_empty() {
            return Err(AppError::Internal(anyhow::anyhow!(
                "CLIProxy 视频任务 ID 不能为空"
            )));
        }
        let url = self.video_url(video_id);
        let v = self.get_json(&url).await?;
        if let Some(message) = failure_message(&v) {
            return Ok(VideoJobState::Failed(message));
        }
        match status(&v).as_deref().unwrap_or_default() {
            "queued" | "pending" | "submitted" | "starting" => Ok(VideoJobState::Pending),
            "in_progress" | "processing" | "running" => Ok(VideoJobState::Running),
            "completed" | "complete" | "succeeded" | "success" | "done" => {
                let result = self.download_completed_video(video_id, v).await?;
                Ok(VideoJobState::Completed(result))
            }
            "" => Err(AppError::Internal(anyhow::anyhow!(
                "CLIProxy 视频任务响应缺少 status"
            ))),
            other => Err(AppError::Internal(anyhow::anyhow!(
                "未知 CLIProxy 视频任务 status={other}"
            ))),
        }
    }

    async fn status(&self, job_id: &str) -> Result<a4n_video::VideoJobStatus, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let video_id = job_id.trim();
        if video_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "CLIProxy 视频任务 ID 不能为空".into(),
            ));
        }
        let value = self.get_status_json(&self.video_url(video_id)).await?;
        let Some(value) = value else {
            return Ok(a4n_video::VideoJobStatus::NotFound);
        };
        let status = status(&value).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("CLIProxy 视频任务响应缺少 status"))
        })?;
        if matches!(
            status.as_str(),
            "queued" | "pending" | "submitted" | "starting"
        ) {
            return Ok(a4n_video::VideoJobStatus::Pending);
        }
        if matches!(status.as_str(), "in_progress" | "processing" | "running") {
            return Ok(a4n_video::VideoJobStatus::Running);
        }
        if matches!(status.as_str(), "cancelled" | "canceled") {
            return Ok(a4n_video::VideoJobStatus::Cancelled);
        }
        if matches!(status.as_str(), "failed" | "failure" | "error" | "expired") {
            return Ok(a4n_video::VideoJobStatus::Failed(
                failure_message(&value).unwrap_or_else(|| "视频生成失败".to_string()),
            ));
        }
        if is_completed_status(Some(status.as_str())) {
            return Ok(a4n_video::VideoJobStatus::Completed);
        }
        Err(AppError::Internal(anyhow::anyhow!(
            "未知 CLIProxy 视频任务 status={status}"
        )))
    }

    async fn delete(&self, job_id: &str) -> Result<VideoJobDeleteOutcome, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let video_id = job_id.trim();
        if video_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "CLIProxy 视频任务 ID 不能为空".into(),
            ));
        }
        let mut request = self.client.delete(self.video_url(video_id));
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
        let response = request.send().await.map_err(|error| {
            AppError::Internal(anyhow::anyhow!("CLIProxy 视频删除失败：{error}"))
        })?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(VideoJobDeleteOutcome::NotFound);
        }
        if response.status().is_success() {
            return Ok(VideoJobDeleteOutcome::Deleted);
        }
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Internal(anyhow::anyhow!(
                "读取 CLIProxy 视频删除错误响应失败：{error}"
            ))
        })?;
        Err(AppError::Internal(anyhow::anyhow!(
            "CLIProxy 视频删除返回 {status}：{}",
            error_body_preview(&body)
        )))
    }
}
