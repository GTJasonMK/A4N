mod request;
mod response;
#[cfg(test)]
mod tests;

use async_trait::async_trait;

use a4n_domain::ProviderError as AppError;
use a4n_video_provider_contract::{
    VideoGenerationResult, VideoJobState, VideoJobStatus, VideoProvider,
};
use a4n_video_sidecar_contract::VideoGenerationRequest;

use self::response::{decode_job_status, decode_result, extract_job_id};
use crate::transport::{self, ApiKeyAuth};

pub(crate) struct HttpVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl HttpVideoProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn generation_url(&self) -> String {
        if self.base_url.ends_with("/videos/generations") {
            self.base_url.clone()
        } else {
            format!("{}/videos/generations", self.base_url)
        }
    }

    fn job_url(&self, job_id: &str) -> String {
        if self.base_url.contains("{job_id}") {
            self.base_url.replace("{job_id}", job_id)
        } else if self.base_url.ends_with("/videos/generations") {
            format!("{}/{}", self.base_url, job_id)
        } else {
            format!("{}/videos/generations/{}", self.base_url, job_id)
        }
    }
}
#[async_trait]
impl VideoProvider for HttpVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let url = self.generation_url();
        let body = Self::request_body(&req);
        tracing::debug!("video generate POST {url} model={}", req.model);
        let v = transport::post_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::Bearer,
            &url,
            &body,
            provider_idempotency_key_sha256,
            "视频生成",
        )
        .await?;
        let job_id = extract_job_id(&v)?;
        let status = v.get("status").map(|_| decode_job_status(&v)).transpose()?;

        match status {
            Some(VideoJobStatus::Failed(message)) => return Err(AppError::Internal(message)),
            Some(VideoJobStatus::Cancelled) => {
                return Err(AppError::Internal("视频任务已取消".into()));
            }
            Some(VideoJobStatus::Pending | VideoJobStatus::Running) => {}
            Some(VideoJobStatus::Completed) | None => {
                if let Some(result) = decode_result(&self.client, &v, job_id.clone()).await? {
                    return Ok(result);
                }
                if status == Some(VideoJobStatus::Completed) {
                    return Err(AppError::Internal(
                        "视频任务已完成，但响应缺少视频产物".into(),
                    ));
                }
            }
            Some(VideoJobStatus::NotFound) => unreachable!("a submission cannot indicate HTTP 404"),
        }

        if let Some(job_id) = job_id {
            return Ok(VideoGenerationResult {
                bytes: Vec::new(),
                format: String::new(),
                provider_job_id: Some(job_id),
                metadata: v,
            });
        }

        Err(AppError::Internal(
            "视频生成响应缺少 data[0].b64_json、data[0].url 或 id".to_string(),
        ))
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
            "视频任务查询",
        )
        .await?;
        match decode_job_status(&v)? {
            VideoJobStatus::Pending => Ok(VideoJobState::Pending),
            VideoJobStatus::Running => Ok(VideoJobState::Running),
            VideoJobStatus::Failed(message) => Ok(VideoJobState::Failed(message)),
            VideoJobStatus::Cancelled => Ok(VideoJobState::Failed("视频任务已取消".into())),
            VideoJobStatus::Completed => decode_result(&self.client, &v, Some(job_id.to_string()))
                .await?
                .map(VideoJobState::Completed)
                .ok_or_else(|| {
                    AppError::Internal(
                        "视频任务已完成，但响应缺少 data[0].b64_json 或 data[0].url".into(),
                    )
                }),
            VideoJobStatus::NotFound => {
                unreachable!("provider status body cannot indicate HTTP 404")
            }
        }
    }

    async fn status(&self, job_id: &str) -> Result<VideoJobStatus, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::Configuration("视频配置缺少 API Base URL".into()));
        }
        let value = transport::get_status_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::Bearer,
            &self.job_url(job_id),
            "视频任务状态查询",
        )
        .await?;
        let Some(value) = value else {
            return Ok(VideoJobStatus::NotFound);
        };
        decode_job_status(&value)
    }
}
