mod request;
mod response;
#[cfg(test)]
mod tests;

use async_trait::async_trait;
use serde_json::Value;

use a4n_domain::ProviderError as AppError;
use a4n_video_provider_contract::{
    VideoGenerationResult, VideoJobState, VideoJobStatus, VideoProvider,
};
use a4n_video_sidecar_contract::VideoGenerationRequest;

use self::response::{extract_status, extract_task_id, failure_message, result_video_url};
use crate::download::download_video_url;
use crate::transport::{self, ApiKeyAuth};

/// Seedance 2.0 provider for the documented `/task/submit` task API.
///
/// Reference media mode uses `functionMode = "omni_reference"` with
/// `image_files` / `video_files` / `audio_files`, referenced from the prompt as
/// `@image_file_N`, `@video_file_N`, and `@audio_file_N`.
pub(crate) struct Seedance2VideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl Seedance2VideoProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Self {
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

    fn task_url(&self, task_id: &str) -> String {
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
    async fn decode_completed(
        &self,
        v: &Value,
        provider_job_id: Option<String>,
    ) -> Result<VideoGenerationResult, AppError> {
        let url = result_video_url(v).ok_or_else(|| {
            AppError::Internal("Seedance 2.0 完成响应缺少 result.video_url".to_string())
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
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let url = self.submit_url();
        let body = Self::request_body(&req)?;
        tracing::debug!("seedance2 video submit POST {url} model={}", req.model);
        let v = transport::post_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
            &url,
            &body,
            provider_idempotency_key_sha256,
            "Seedance 2.0 视频生成",
        )
        .await?;
        if matches!(
            extract_status(&v).map(str::to_ascii_lowercase).as_deref(),
            Some("completed")
        ) {
            return self.decode_completed(&v, extract_task_id(&v)).await;
        }
        let task_id = extract_task_id(&v).ok_or_else(|| {
            AppError::Internal("Seedance 2.0 视频生成响应缺少 task_id".to_string())
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
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let url = self.task_url(job_id);
        let v = transport::get_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
            &url,
            "Seedance 2.0 视频任务查询",
        )
        .await?;
        let status = extract_status(&v)
            .ok_or_else(|| AppError::Internal("Seedance 2.0 任务响应缺少 status".to_string()))?
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
            other => Err(AppError::Internal(format!(
                "未知 Seedance 2.0 任务状态：{other}"
            ))),
        }
    }

    async fn status(&self, job_id: &str) -> Result<VideoJobStatus, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let value = transport::get_status_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
            &self.task_url(job_id),
            "Seedance 2.0 视频任务状态查询",
        )
        .await?;
        let Some(value) = value else {
            return Ok(VideoJobStatus::NotFound);
        };
        let status = extract_status(&value)
            .ok_or_else(|| AppError::Internal("Seedance 2.0 任务响应缺少 status".to_string()))?
            .to_ascii_lowercase();
        match status.as_str() {
            "pending" | "queued" | "submitted" | "starting" => Ok(VideoJobStatus::Pending),
            "processing" | "running" | "in_progress" => Ok(VideoJobStatus::Running),
            "completed" | "succeeded" | "success" | "done" => Ok(VideoJobStatus::Completed),
            "cancelled" | "canceled" => Ok(VideoJobStatus::Cancelled),
            "failed" | "error" | "failure" | "expired" => {
                Ok(VideoJobStatus::Failed(failure_message(&value)))
            }
            other => Err(AppError::Internal(format!(
                "未知 Seedance 2.0 任务状态：{other}"
            ))),
        }
    }
}
