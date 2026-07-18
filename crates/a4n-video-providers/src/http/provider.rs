use async_trait::async_trait;
use serde_json::Value;

use a4n_core::error::AppError;
use a4n_video::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

use super::super::response::parse_json_response;
use super::response::{decode_result, extract_job_id, required_string};

pub struct HttpVideoProvider {
    base_url: String,
    api_key: String,
    pub(super) client: reqwest::Client,
}

impl HttpVideoProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
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
impl HttpVideoProvider {
    async fn send_json(&self, url: &str, body: Value) -> Result<Value, AppError> {
        let mut req = self.client.post(url).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("视频生成请求失败：{e}")))?;
        parse_json_response(resp, "视频生成").await
    }

    async fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let mut req = self.client.get(url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("视频任务查询失败：{e}")))?;
        parse_json_response(resp, "视频任务查询").await
    }
}

#[async_trait]
impl VideoProvider for HttpVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.generation_url();
        let body = Self::request_body(&req);
        tracing::debug!("video generate POST {url} model={}", req.model);
        let v = self.send_json(&url, body).await?;
        let job_id = extract_job_id(&v)?;

        if let Some(result) = decode_result(&self.client, &v, job_id.clone()).await? {
            return Ok(result);
        }

        if let Some(job_id) = job_id {
            return Ok(VideoGenerationResult {
                bytes: Vec::new(),
                format: String::new(),
                provider_job_id: Some(job_id),
                metadata: v,
            });
        }

        Err(AppError::Internal(anyhow::anyhow!(
            "视频生成响应缺少 data[0].b64_json、data[0].url 或 id"
        )))
    }

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.job_url(job_id);
        let v = self.get_json(&url).await?;
        let status = required_string(&v, "/status", "视频任务 status")?.to_lowercase();

        if let Some(result) = decode_result(&self.client, &v, Some(job_id.to_string())).await? {
            return Ok(VideoJobState::Completed(result));
        }

        if matches!(status.as_str(), "failed" | "cancelled") {
            let msg = required_string(&v, "/error_message", "视频任务 error_message")?
                .chars()
                .take(500)
                .collect::<String>();
            return Ok(VideoJobState::Failed(msg));
        }

        if status == "pending" {
            return Ok(VideoJobState::Pending);
        }

        if status == "running" {
            return Ok(VideoJobState::Running);
        }

        if status == "completed" {
            return Err(AppError::Internal(anyhow::anyhow!(
                "视频任务已完成，但响应缺少 data[0].b64_json 或 data[0].url"
            )));
        }

        Err(AppError::Internal(anyhow::anyhow!(
            "未知视频任务 status={status}"
        )))
    }
}
