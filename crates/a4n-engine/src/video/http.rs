use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Map, Value, json};

use a4n_core::error::AppError;

use super::download::download_video_url;
use super::response::parse_json_response;
use super::types::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

pub struct HttpVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
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

    fn request_body(req: &VideoGenerationRequest) -> Value {
        let mut body = Map::new();
        body.insert("model".into(), json!(req.model));
        body.insert("prompt".into(), json!(req.prompt));
        body.insert("negative_prompt".into(), json!(req.negative_prompt));
        body.insert("width".into(), json!(req.width));
        body.insert("height".into(), json!(req.height));
        body.insert("duration_secs".into(), json!(req.duration_secs));
        body.insert("fps".into(), json!(req.fps));
        body.insert(
            "reference_image_urls".into(),
            json!(req.reference_image_urls.clone()),
        );
        if let Some(image) = req.first_frame_image_url.as_ref() {
            body.insert("image".into(), json!({ "url": image }));
            body.insert("first_frame_image_url".into(), json!(image));
        }
        if let Some(image) = req.last_frame_image_url.as_ref() {
            body.insert("last_frame".into(), json!({ "url": image }));
            body.insert("last_frame_image_url".into(), json!(image));
        }
        body.insert("response_format".into(), json!("b64_json"));
        Value::Object(body)
    }

    pub fn request_body_preview(req: &VideoGenerationRequest) -> Value {
        Self::request_body(req)
    }

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

    async fn decode_result(
        &self,
        v: &Value,
        provider_job_id: Option<String>,
    ) -> Result<Option<VideoGenerationResult>, AppError> {
        let Some(item) = first_data_item(v)? else {
            return Ok(None);
        };
        let format = required_video_format(item)?;

        if let Some(b64) = optional_string(item, "b64_json", "data[0].b64_json")? {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("视频 base64 解码失败：{e}")))?;
            return Ok(Some(VideoGenerationResult {
                bytes,
                format,
                provider_job_id,
                metadata: v.clone(),
            }));
        }

        if let Some(url) = optional_string(item, "url", "data[0].url")? {
            let (bytes, format) = download_video_url(&self.client, url, Some(&format)).await?;
            return Ok(Some(VideoGenerationResult {
                bytes,
                format,
                provider_job_id,
                metadata: v.clone(),
            }));
        }

        Ok(None)
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

        if let Some(result) = self.decode_result(&v, job_id.clone()).await? {
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

        if let Some(result) = self.decode_result(&v, Some(job_id.to_string())).await? {
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

fn first_data_item(v: &Value) -> Result<Option<&Value>, AppError> {
    let Some(data) = v.get("data") else {
        return Ok(None);
    };
    let arr = data
        .as_array()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("视频响应 data 必须是数组")))?;
    let Some(item) = arr.first() else {
        return Ok(None);
    };
    if !item.is_object() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "视频响应 data[0] 必须是对象"
        )));
    }
    Ok(Some(item))
}

fn optional_string<'a>(v: &'a Value, key: &str, label: &str) -> Result<Option<&'a str>, AppError> {
    let Some(value) = v.get(key) else {
        return Ok(None);
    };
    let text = value
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("{label} 必须是非空字符串")))?;
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{label} 必须是非空字符串"
        )));
    }
    Ok(Some(text))
}

fn required_string<'a>(v: &'a Value, pointer: &str, label: &str) -> Result<&'a str, AppError> {
    let value = v
        .pointer(pointer)
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("视频响应缺少 {label}")))?;
    let text = value
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("{label} 必须是非空字符串")))?;
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{label} 必须是非空字符串"
        )));
    }
    Ok(text)
}

fn required_video_format(item: &Value) -> Result<String, AppError> {
    let raw = optional_string(item, "format", "data[0].format")?
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("视频响应缺少 data[0].format")))?;
    let format = raw.trim().to_ascii_lowercase();
    if matches!(format.as_str(), "mp4" | "webm" | "mov" | "m4v") {
        Ok(format)
    } else {
        Err(AppError::Internal(anyhow::anyhow!(
            "不支持的视频格式：{format}"
        )))
    }
}

fn extract_job_id(v: &Value) -> Result<Option<String>, AppError> {
    let Some(value) = v.get("id") else {
        return Ok(None);
    };
    let id = value
        .as_str()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("视频生成响应 id 必须是非空字符串")))?;
    let id = id.trim();
    if id.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "视频生成响应 id 必须是非空字符串"
        )));
    }
    Ok(Some(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn provider() -> HttpVideoProvider {
        HttpVideoProvider::new("http://127.0.0.1:9", "test-key")
    }

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            prompt: "animate".into(),
            negative_prompt: Some("noise".into()),
            width: 1280,
            height: 720,
            duration_secs: 5,
            fps: 24,
            model: "video-model".into(),
            first_frame_image_url: Some("https://example.test/first.png".into()),
            last_frame_image_url: Some("https://example.test/last.png".into()),
            reference_image_urls: vec!["https://example.test/ref.png".into()],
            reference_media: None,
        }
    }

    #[test]
    fn request_body_keeps_reference_images_with_boundary_frames() {
        let body = HttpVideoProvider::request_body(&request());

        assert_eq!(body["image"]["url"], "https://example.test/first.png");
        assert_eq!(body["last_frame"]["url"], "https://example.test/last.png");
        assert_eq!(
            body["reference_image_urls"][0],
            "https://example.test/ref.png"
        );
    }

    #[tokio::test]
    async fn decodes_canonical_b64_video_result() {
        let raw = base64::engine::general_purpose::STANDARD.encode(b"video-bytes");
        let value = json!({
            "data": [{
                "b64_json": raw,
                "format": "mp4"
            }]
        });

        let result = provider()
            .decode_result(&value, None)
            .await
            .expect("decode succeeds")
            .expect("has result");

        assert_eq!(result.bytes, b"video-bytes");
        assert_eq!(result.format, "mp4");
    }

    #[tokio::test]
    async fn rejects_b64_video_without_format() {
        let raw = base64::engine::general_purpose::STANDARD.encode(b"video-bytes");
        let value = json!({
            "data": [{
                "b64_json": raw
            }]
        });

        assert!(provider().decode_result(&value, None).await.is_err());
    }

    #[test]
    fn extracts_only_top_level_id() {
        assert_eq!(
            extract_job_id(&json!({"id": "job-1"})).unwrap().as_deref(),
            Some("job-1")
        );
        assert!(
            extract_job_id(&json!({"job_id": "job-1"}))
                .unwrap()
                .is_none()
        );
        assert!(
            extract_job_id(&json!({"data": {"id": "job-1"}}))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn requires_top_level_status() {
        assert_eq!(
            required_string(&json!({"status": "running"}), "/status", "视频任务 status").unwrap(),
            "running"
        );
        assert!(
            required_string(&json!({"state": "running"}), "/status", "视频任务 status").is_err()
        );
    }

    #[test]
    fn rejects_unknown_video_format() {
        let value = json!({"format": "avi"});
        assert!(required_video_format(&value).is_err());
    }
}
