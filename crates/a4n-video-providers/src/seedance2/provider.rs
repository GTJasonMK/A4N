use async_trait::async_trait;
use serde_json::Value;

use a4n_core::error::AppError;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

use super::super::download::download_video_url;
use super::super::response::{parse_json_response, parse_json_response_allow_not_found};
use super::response::{extract_status, extract_task_id, failure_message, result_video_url};

/// Seedance 2.0 provider for the documented `/task/submit` task API.
///
/// Reference media mode uses `functionMode = "omni_reference"` with
/// `image_files` / `video_files` / `audio_files`, referenced from the prompt as
/// `@image_file_N`, `@video_file_N`, and `@audio_file_N`.
pub struct Seedance2VideoProvider {
    base_url: String,
    api_key: String,
    pub(super) client: reqwest::Client,
}

impl Seedance2VideoProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn submit_url(&self) -> String {
        if self.base_url.ends_with("/task/submit") {
            self.base_url.clone()
        } else {
            format!("{}/task/submit", self.base_url)
        }
    }

    pub(super) fn task_url(&self, task_id: &str) -> String {
        if self.base_url.contains("{task_id}") {
            return self.base_url.replace("{task_id}", task_id);
        }
        if let Some(root) = self.base_url.strip_suffix("/task/submit") {
            return format!("{root}/task/{task_id}");
        }
        format!("{}/task/{task_id}", self.base_url)
    }
}
impl Seedance2VideoProvider {
    async fn send_json(
        &self,
        url: &str,
        body: Value,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<Value, AppError> {
        let mut req = self.client.post(url).json(&body);
        if !self.api_key.trim().is_empty() {
            req = req.bearer_auth(self.api_key.trim());
        }
        let req = crate::idempotency::with_provider_idempotency_key(
            req,
            provider_idempotency_key_sha256,
        )?;
        let resp = req.send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Seedance 2.0 视频生成请求失败：{e}"))
        })?;
        parse_json_response(resp, "Seedance 2.0 视频生成").await
    }

    async fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let mut req = self.client.get(url);
        if !self.api_key.trim().is_empty() {
            req = req.bearer_auth(self.api_key.trim());
        }
        let resp = req.send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Seedance 2.0 视频任务查询失败：{e}"))
        })?;
        parse_json_response(resp, "Seedance 2.0 视频任务查询").await
    }

    async fn get_status_json(&self, url: &str) -> Result<Option<Value>, AppError> {
        let mut req = self.client.get(url);
        if !self.api_key.trim().is_empty() {
            req = req.bearer_auth(self.api_key.trim());
        }
        let resp = req.send().await.map_err(|e| {
            AppError::Internal(anyhow::anyhow!("Seedance 2.0 视频任务状态查询失败：{e}"))
        })?;
        parse_json_response_allow_not_found(resp, "Seedance 2.0 视频任务状态查询").await
    }

    pub(super) async fn decode_completed(
        &self,
        v: &Value,
        provider_job_id: Option<String>,
    ) -> Result<VideoGenerationResult, AppError> {
        let url = result_video_url(v).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Seedance 2.0 完成响应缺少 result.video_url"
            ))
        })?;
        let (bytes, format) = download_video_url(&self.client, url, None).await?;
        Ok(VideoGenerationResult {
            bytes,
            format,
            provider_job_id,
            metadata: v.clone(),
        })
    }
}

#[async_trait]
impl VideoProvider for Seedance2VideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.submit_url();
        let body = Self::request_body(&req)?;
        tracing::debug!("seedance2 video submit POST {url} model={}", req.model);
        let v = self
            .send_json(&url, body, provider_idempotency_key_sha256)
            .await?;
        if matches!(
            extract_status(&v).map(str::to_ascii_lowercase).as_deref(),
            Some("completed")
        ) {
            return self.decode_completed(&v, extract_task_id(&v)).await;
        }
        let task_id = extract_task_id(&v).ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!("Seedance 2.0 视频生成响应缺少 task_id"))
        })?;
        Ok(VideoGenerationResult {
            bytes: Vec::new(),
            format: String::new(),
            provider_job_id: Some(task_id),
            metadata: v,
        })
    }

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.task_url(job_id);
        let v = self.get_json(&url).await?;
        let status = extract_status(&v)
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Seedance 2.0 任务响应缺少 status")))?
            .to_ascii_lowercase();
        match status.as_str() {
            "pending" | "queued" | "submitted" => Ok(VideoJobState::Pending),
            "processing" | "running" | "in_progress" => Ok(VideoJobState::Running),
            "completed" | "succeeded" | "success" => self
                .decode_completed(&v, Some(job_id.to_string()))
                .await
                .map(VideoJobState::Completed),
            "failed" | "error" | "cancelled" | "canceled" => {
                Ok(VideoJobState::Failed(failure_message(&v)))
            }
            other => Err(AppError::Internal(anyhow::anyhow!(
                "未知 Seedance 2.0 任务状态：{other}"
            ))),
        }
    }

    async fn status(&self, job_id: &str) -> Result<a4n_video::VideoJobStatus, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let value = self.get_status_json(&self.task_url(job_id)).await?;
        let Some(value) = value else {
            return Ok(a4n_video::VideoJobStatus::NotFound);
        };
        let status = extract_status(&value)
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("Seedance 2.0 任务响应缺少 status")))?
            .to_ascii_lowercase();
        match status.as_str() {
            "pending" | "queued" | "submitted" | "starting" => {
                Ok(a4n_video::VideoJobStatus::Pending)
            }
            "processing" | "running" | "in_progress" => Ok(a4n_video::VideoJobStatus::Running),
            "completed" | "succeeded" | "success" | "done" => {
                Ok(a4n_video::VideoJobStatus::Completed)
            }
            "cancelled" | "canceled" => Ok(a4n_video::VideoJobStatus::Cancelled),
            "failed" | "error" | "failure" | "expired" => {
                Ok(a4n_video::VideoJobStatus::Failed(failure_message(&value)))
            }
            other => Err(AppError::Internal(anyhow::anyhow!(
                "未知 Seedance 2.0 任务状态：{other}"
            ))),
        }
    }
}
