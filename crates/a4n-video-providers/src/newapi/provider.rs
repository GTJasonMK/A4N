use super::request::{aspect_ratio_from_size, size_from_request};
use super::response::{
    api_error_suffix, extract_failure_message, extract_status, extract_task_id, find_video_b64,
    find_video_url,
};
use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Map, Value, json};

use a4n_domain::ProviderError as AppError;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

use crate::download::download_video_url;
use crate::format::{detect_video_format_from_bytes, normalize_video_format_str};
use crate::transport::{self, ApiKeyAuth};

pub(crate) struct NewApiVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl NewApiVideoProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn generation_url(&self) -> String {
        if self.base_url.ends_with("/video/generations") {
            self.base_url.clone()
        } else {
            format!("{}/video/generations", self.base_url)
        }
    }

    fn job_url(&self, task_id: &str) -> String {
        if self.base_url.contains("{task_id}") {
            self.base_url.replace("{task_id}", task_id)
        } else if self.base_url.contains("{job_id}") {
            self.base_url.replace("{job_id}", task_id)
        } else if self.base_url.ends_with("/video/generations") {
            format!("{}/{}", self.base_url, task_id)
        } else {
            format!("{}/video/generations/{}", self.base_url, task_id)
        }
    }

    pub(crate) fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        req.validate_dimensions()
            .map_err(AppError::InvalidArgument)?;
        if req.media.last_frame().is_some() {
            return Err(AppError::InvalidArgument(
                "New API 视频接口只有一个 image 输入槽，只能作为首帧使用，不支持尾帧输入"
                    .to_string(),
            ));
        }
        if !req.media.reference_images().is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "New API 视频接口不支持素材参考图输入，当前收到 {} 张参考图",
                req.media.reference_images().len()
            )));
        }
        let image_input = req.media.first_frame();
        let mut body = Map::new();
        body.insert("model".into(), json!(req.model));
        body.insert("prompt".into(), json!(req.prompt));
        body.insert("duration".into(), json!(req.duration_secs));
        body.insert("size".into(), json!(size_from_request(req)));
        let aspect_ratio = aspect_ratio_from_size(req.width, req.height);
        let negative_prompt = req
            .negative_prompt
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty());
        if let Some(aspect_ratio) = aspect_ratio {
            body.insert("aspect_ratio".into(), json!(aspect_ratio));
        }
        if let Some(negative_prompt) = negative_prompt {
            body.insert("negative_prompt".into(), json!(negative_prompt));
        }

        if let Some(image) = image_input {
            body.insert("image".into(), json!(image));
        }

        Ok(Value::Object(body))
    }

    async fn decode_result(
        &self,
        v: &Value,
        provider_job_id: Option<String>,
    ) -> Result<Option<VideoGenerationResult>, AppError> {
        if let Some(payload) = find_video_b64(v)? {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload.b64)
                .map_err(|e| AppError::Internal(format!("视频 base64 解码失败：{e}")))?;
            let format = match payload.format {
                Some(format) => normalize_video_format_str(format)?,
                None => detect_video_format_from_bytes(&bytes)?,
            };
            return Ok(Some(VideoGenerationResult {
                bytes,
                format,
                provider_job_id,
                metadata: v.clone(),
            }));
        }

        let Some(payload) = find_video_url(v)? else {
            return Ok(None);
        };
        let (bytes, format) = download_video_url(&self.client, payload.url, payload.format).await?;

        Ok(Some(VideoGenerationResult {
            bytes,
            format,
            provider_job_id,
            metadata: v.clone(),
        }))
    }
}

#[async_trait]
impl VideoProvider for NewApiVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let url = self.generation_url();
        let body = Self::request_body(&req)?;
        tracing::debug!("newapi video generate POST {url} model={}", req.model);
        let v = transport::post_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::Bearer,
            &url,
            &body,
            provider_idempotency_key_sha256,
            "New API 视频生成",
        )
        .await?;
        let task_id = extract_task_id(&v)?;

        if let Some(result) = self.decode_result(&v, task_id.clone()).await? {
            return Ok(result);
        }
        if let Some(task_id) = task_id {
            return Ok(VideoGenerationResult {
                bytes: Vec::new(),
                format: String::new(),
                provider_job_id: Some(task_id),
                metadata: v,
            });
        }

        Err(AppError::Internal(format!(
            "New API 视频生成响应缺少 task_id/id{}",
            api_error_suffix(&v)
        )))
    }

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let url = self.job_url(job_id);
        let v = transport::get_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::Bearer,
            &url,
            "New API 视频任务查询",
        )
        .await?;

        if let Some(result) = self.decode_result(&v, Some(job_id.to_string())).await? {
            return Ok(VideoJobState::Completed(result));
        }

        let status = extract_status(&v)
            .ok_or_else(|| AppError::Internal("New API 视频任务响应缺少 status".to_string()))?
            .to_ascii_lowercase();
        if matches!(
            status.as_str(),
            "queued" | "pending" | "submitted" | "starting"
        ) {
            return Ok(VideoJobState::Pending);
        }
        if matches!(status.as_str(), "running" | "in_progress" | "processing") {
            return Ok(VideoJobState::Running);
        }
        if matches!(
            status.as_str(),
            "completed" | "complete" | "succeeded" | "success" | "done"
        ) {
            return Err(AppError::Internal(
                "New API 视频任务已完成，但响应缺少可下载视频 URL 或 b64_json".to_string(),
            ));
        }
        if matches!(
            status.as_str(),
            "failed" | "failure" | "cancelled" | "canceled" | "expired"
        ) {
            return Ok(VideoJobState::Failed(extract_failure_message(&v)));
        }

        Err(AppError::Internal(format!(
            "未知 New API 视频任务 status={status}"
        )))
    }

    async fn status(&self, job_id: &str) -> Result<a4n_video::VideoJobStatus, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let value = transport::get_status_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::Bearer,
            &self.job_url(job_id),
            "New API 视频任务状态查询",
        )
        .await?;
        let Some(value) = value else {
            return Ok(a4n_video::VideoJobStatus::NotFound);
        };
        let status = extract_status(&value)
            .ok_or_else(|| AppError::Internal("New API 视频任务响应缺少 status".to_string()))?
            .to_ascii_lowercase();
        if matches!(
            status.as_str(),
            "queued" | "pending" | "submitted" | "starting"
        ) {
            return Ok(a4n_video::VideoJobStatus::Pending);
        }
        if matches!(status.as_str(), "running" | "in_progress" | "processing") {
            return Ok(a4n_video::VideoJobStatus::Running);
        }
        if matches!(status.as_str(), "cancelled" | "canceled") {
            return Ok(a4n_video::VideoJobStatus::Cancelled);
        }
        if matches!(status.as_str(), "failed" | "failure" | "expired" | "error") {
            return Ok(a4n_video::VideoJobStatus::Failed(extract_failure_message(
                &value,
            )));
        }
        if matches!(
            status.as_str(),
            "completed" | "complete" | "succeeded" | "success" | "done"
        ) {
            return Ok(a4n_video::VideoJobStatus::Completed);
        }
        Err(AppError::Internal(format!(
            "未知 New API 视频任务 status={status}"
        )))
    }
}
