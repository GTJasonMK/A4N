use super::request::request_body_for_route;
use super::response::{
    completed_video_url, error_body_preview, failure_message, is_completed_status, required_status,
    status, url_escape_path_segment, video_id,
};
use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde_json::Value;

use a4n_domain::ProviderError as AppError;
use a4n_video::{
    VideoGenerationRequest, VideoGenerationResult, VideoJobDeleteOutcome, VideoJobState,
    VideoProvider,
};

use crate::download::download_video_url;
use crate::format::video_format_from_content_type_or_bytes;
use crate::transport::{self, ApiKeyAuth};

/// GGGB Gateway 原生 `/v1/videos/generations` 与 OpenAI `/openai/v1/videos` 适配器。
pub(crate) struct CliProxyOpenAiVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl CliProxyOpenAiVideoProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn videos_url(&self) -> String {
        if self.base_url.ends_with("/videos/generations") {
            self.base_url.trim_end_matches("/generations").to_string()
        } else if self.base_url.ends_with("/videos") {
            self.base_url.clone()
        } else {
            format!("{}/videos", self.base_url)
        }
    }

    fn create_url(&self) -> String {
        if self.uses_openai_route() {
            self.videos_url()
        } else {
            format!("{}/generations", self.videos_url())
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

    #[cfg(test)]
    pub(super) fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        request_body_for_route(req, true)
    }

    fn request_body_for_config(&self, req: &VideoGenerationRequest) -> Result<Value, AppError> {
        request_body_for_route(req, self.uses_openai_route())
    }

    pub(crate) fn request_body_preview_for_base_url(
        req: &VideoGenerationRequest,
        base_url: &str,
    ) -> Result<Value, AppError> {
        Self::new(base_url, "").request_body_for_config(req)
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

        self.download_content(video_id, metadata).await
    }

    pub(super) fn uses_openai_route(&self) -> bool {
        self.videos_url().contains("/openai/")
    }

    async fn download_content(
        &self,
        video_id: &str,
        metadata: Value,
    ) -> Result<VideoGenerationResult, AppError> {
        let url = self.content_url(video_id);
        let request = transport::apply_api_key(
            self.client.get(&url),
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
        );
        let resp = request
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("CLIProxy 视频下载请求失败：{e}")))?;
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.map_err(|e| {
                AppError::Internal(format!("读取 CLIProxy 视频下载错误响应失败：{e}"))
            })?;
            return Err(AppError::Internal(format!(
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
            .map_err(|e| AppError::Internal(format!("读取 CLIProxy 视频字节失败：{e}")))?
            .to_vec();
        let format = video_format_from_content_type_or_bytes(content_type.as_deref(), &bytes)
            .ok_or_else(|| {
                AppError::Internal(
                    "CLIProxy 视频响应缺少可识别的 Content-Type，且视频字节格式无法识别"
                        .to_string(),
                )
            })?;
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
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let url = self.create_url();
        let body = self.request_body_for_config(&req)?;
        tracing::debug!("cliproxy video POST {url} model={}", req.model);
        let v = transport::post_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
            &url,
            &body,
            provider_idempotency_key_sha256,
            "CLIProxy 视频生成",
        )
        .await?;
        let video_id = video_id(&v)?;

        if is_completed_status(status(&v).as_deref()) {
            return self.download_completed_video(&video_id, v).await;
        }
        if let Some(message) = failure_message(&v) {
            return Err(AppError::Internal(format!(
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
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let video_id = job_id.trim();
        if video_id.is_empty() {
            return Err(AppError::Internal(
                "CLIProxy 视频任务 ID 不能为空".to_string(),
            ));
        }
        let url = self.video_url(video_id);
        let v = transport::get_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
            &url,
            "CLIProxy 视频任务查询",
        )
        .await?;
        if let Some(message) = failure_message(&v) {
            return Ok(VideoJobState::Failed(message));
        }
        let status = required_status(&v)?;
        match status.as_str() {
            "queued" | "pending" | "submitted" | "starting" => Ok(VideoJobState::Pending),
            "in_progress" | "processing" | "running" => Ok(VideoJobState::Running),
            "completed" | "complete" | "succeeded" | "success" | "done" => {
                let result = self.download_completed_video(video_id, v).await?;
                Ok(VideoJobState::Completed(result))
            }
            other => Err(AppError::Internal(format!(
                "未知 CLIProxy 视频任务 status={other}"
            ))),
        }
    }

    async fn status(&self, job_id: &str) -> Result<a4n_video::VideoJobStatus, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let video_id = job_id.trim();
        if video_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "CLIProxy 视频任务 ID 不能为空".into(),
            ));
        }
        let value = transport::get_status_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
            &self.video_url(video_id),
            "CLIProxy 视频任务状态查询",
        )
        .await?;
        let Some(value) = value else {
            return Ok(a4n_video::VideoJobStatus::NotFound);
        };
        let status = required_status(&value)?;
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
        Err(AppError::Internal(format!(
            "未知 CLIProxy 视频任务 status={status}"
        )))
    }

    async fn delete(&self, job_id: &str) -> Result<VideoJobDeleteOutcome, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let video_id = job_id.trim();
        if video_id.is_empty() {
            return Err(AppError::InvalidArgument(
                "CLIProxy 视频任务 ID 不能为空".into(),
            ));
        }
        let request = transport::apply_api_key(
            self.client.delete(self.video_url(video_id)),
            &self.api_key,
            ApiKeyAuth::TrimmedBearer,
        );
        let response = request
            .send()
            .await
            .map_err(|error| AppError::Internal(format!("CLIProxy 视频删除失败：{error}")))?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(VideoJobDeleteOutcome::NotFound);
        }
        if response.status().is_success() {
            return Ok(VideoJobDeleteOutcome::Deleted);
        }
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Internal(format!("读取 CLIProxy 视频删除错误响应失败：{error}"))
        })?;
        Err(AppError::Internal(format!(
            "CLIProxy 视频删除返回 {status}：{}",
            error_body_preview(&body)
        )))
    }
}
