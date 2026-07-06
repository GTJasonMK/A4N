use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Map, Value, json};

use a4n_core::error::AppError;

use super::download::download_video_url;
use super::format::{detect_video_format_from_bytes, normalize_video_format_str};
use super::response::parse_json_response;
use super::types::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

/// New API 视频 provider：使用单数 `/v1/video/generations` 任务接口。
pub struct NewApiVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl NewApiVideoProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
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

    fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        let is_veo = is_veo_model(&req.model);
        if req.last_frame_image_url.is_some() {
            return Err(AppError::InvalidArgument(
                "New API 视频接口只有一个 image 输入槽，只能作为首帧使用，不支持尾帧输入"
                    .to_string(),
            ));
        }
        if !req.reference_image_urls.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "New API 视频接口不支持素材参考图输入，当前收到 {} 张参考图",
                req.reference_image_urls.len()
            )));
        }
        let image_input = req.first_frame_image_url.clone();
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
        if is_veo {
            body.insert("duration_seconds".into(), json!(req.duration_secs));
            body.insert("number_of_videos".into(), json!(1));
            body.insert("enhance_prompt".into(), json!(true));
            body.insert(
                "metadata".into(),
                veo_metadata(req, aspect_ratio, negative_prompt),
            );
        }
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

    pub fn request_body_preview(req: &VideoGenerationRequest) -> Result<Value, AppError> {
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
            .map_err(|e| AppError::Internal(anyhow::anyhow!("New API 视频生成请求失败：{e}")))?;
        parse_json_response(resp, "New API 视频生成").await
    }

    async fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let mut req = self.client.get(url);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("New API 视频任务查询失败：{e}")))?;
        parse_json_response(resp, "New API 视频任务查询").await
    }

    async fn decode_result(
        &self,
        v: &Value,
        provider_job_id: Option<String>,
    ) -> Result<Option<VideoGenerationResult>, AppError> {
        if let Some(payload) = find_video_b64(v)? {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload.b64)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("视频 base64 解码失败：{e}")))?;
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

fn is_veo_model(model: &str) -> bool {
    model.trim().to_ascii_lowercase().contains("veo")
}

fn size_from_request(req: &VideoGenerationRequest) -> String {
    format!("{}x{}", req.width.max(1), req.height.max(1))
}

fn veo_metadata(
    req: &VideoGenerationRequest,
    aspect_ratio: Option<&str>,
    negative_prompt: Option<&str>,
) -> Value {
    let mut metadata = Map::new();
    metadata.insert("durationSeconds".into(), json!(req.duration_secs));
    metadata.insert(
        "resolution".into(),
        json!(veo_resolution_from_size(req.width, req.height)),
    );
    if let Some(aspect_ratio) = aspect_ratio {
        metadata.insert("aspectRatio".into(), json!(aspect_ratio));
    }
    if let Some(negative_prompt) = negative_prompt {
        metadata.insert("negativePrompt".into(), json!(negative_prompt));
    }
    Value::Object(metadata)
}

fn veo_resolution_from_size(width: i32, height: i32) -> &'static str {
    let max_dim = width.max(height);
    if max_dim >= 3840 {
        "4k"
    } else if max_dim >= 1920 {
        "1080p"
    } else {
        "720p"
    }
}

fn aspect_ratio_from_size(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width == height {
        return Some("1:1");
    }
    if width * 9 == height * 16 {
        return Some("16:9");
    }
    if width * 16 == height * 9 {
        return Some("9:16");
    }
    None
}

#[async_trait]
impl VideoProvider for NewApiVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.generation_url();
        let body = Self::request_body(&req)?;
        tracing::debug!("newapi video generate POST {url} model={}", req.model);
        let v = self.send_json(&url, body).await?;
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

        Err(AppError::Internal(anyhow::anyhow!(
            "New API 视频生成响应缺少 task_id/id{}",
            api_error_suffix(&v)
        )))
    }

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.job_url(job_id);
        let v = self.get_json(&url).await?;

        if let Some(result) = self.decode_result(&v, Some(job_id.to_string())).await? {
            return Ok(VideoJobState::Completed(result));
        }

        let status = extract_status(&v)
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("New API 视频任务响应缺少 status")))?
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
            return Err(AppError::Internal(anyhow::anyhow!(
                "New API 视频任务已完成，但响应缺少可下载视频 URL 或 b64_json"
            )));
        }
        if matches!(
            status.as_str(),
            "failed" | "failure" | "cancelled" | "canceled" | "expired"
        ) {
            return Ok(VideoJobState::Failed(extract_failure_message(&v)));
        }

        Err(AppError::Internal(anyhow::anyhow!(
            "未知 New API 视频任务 status={status}"
        )))
    }
}

struct VideoUrlPayload<'a> {
    url: &'a str,
    format: Option<&'a str>,
}

struct VideoB64Payload<'a> {
    b64: &'a str,
    format: Option<&'a str>,
}

fn extract_task_id(v: &Value) -> Result<Option<String>, AppError> {
    for pointer in ["/task_id", "/id", "/data/task_id", "/data/id"] {
        if let Some(text) = optional_pointer_str(v, pointer)? {
            return Ok(Some(text.to_string()));
        }
    }
    Ok(None)
}

fn extract_status(v: &Value) -> Option<&str> {
    [
        "/data/data/status",
        "/data/status",
        "/status",
        "/state",
        "/data/state",
    ]
    .into_iter()
    .find_map(|pointer| v.pointer(pointer).and_then(Value::as_str).map(str::trim))
    .filter(|text| !text.is_empty())
}

fn find_video_url(v: &Value) -> Result<Option<VideoUrlPayload<'_>>, AppError> {
    for (url_pointer, format_pointers) in [
        (
            "/data/data/video/url",
            &["/data/data/video/format", "/data/data/format"][..],
        ),
        ("/data/data/url", &["/data/data/format"][..]),
        (
            "/data/video/url",
            &["/data/video/format", "/data/format"][..],
        ),
        ("/data/url", &["/data/format"][..]),
        ("/video/url", &["/video/format", "/format"][..]),
        ("/url", &["/format"][..]),
    ] {
        let Some(url) = optional_pointer_str(v, url_pointer)? else {
            continue;
        };
        if !looks_like_http_url(url) {
            continue;
        }
        let format = first_optional_pointer_str(v, format_pointers)?;
        return Ok(Some(VideoUrlPayload { url, format }));
    }
    Ok(None)
}

fn find_video_b64(v: &Value) -> Result<Option<VideoB64Payload<'_>>, AppError> {
    for (b64_pointer, format_pointers) in [
        (
            "/data/data/video/b64_json",
            &["/data/data/video/format", "/data/data/format"][..],
        ),
        ("/data/data/b64_json", &["/data/data/format"][..]),
        (
            "/data/video/b64_json",
            &["/data/video/format", "/data/format"][..],
        ),
        ("/data/b64_json", &["/data/format"][..]),
        ("/video/b64_json", &["/video/format", "/format"][..]),
        ("/b64_json", &["/format"][..]),
    ] {
        let Some(b64) = optional_pointer_str(v, b64_pointer)? else {
            continue;
        };
        let format = first_optional_pointer_str(v, format_pointers)?;
        return Ok(Some(VideoB64Payload { b64, format }));
    }
    Ok(None)
}

fn optional_pointer_str<'a>(v: &'a Value, pointer: &str) -> Result<Option<&'a str>, AppError> {
    let Some(value) = v.pointer(pointer) else {
        return Ok(None);
    };
    let text = value.as_str().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!(
            "New API 视频响应字段 {pointer} 必须是字符串"
        ))
    })?;
    let text = text.trim();
    if text.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "New API 视频响应字段 {pointer} 必须是非空字符串"
        )));
    }
    Ok(Some(text))
}

fn first_optional_pointer_str<'a>(
    v: &'a Value,
    pointers: &[&str],
) -> Result<Option<&'a str>, AppError> {
    for pointer in pointers {
        if let Some(text) = optional_pointer_str(v, pointer)? {
            return Ok(Some(text));
        }
    }
    Ok(None)
}

fn looks_like_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

fn api_error_suffix(v: &Value) -> String {
    v.pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| format!("；错误：{}", text.chars().take(500).collect::<String>()))
        .unwrap_or_default()
}

fn extract_failure_message(v: &Value) -> String {
    for pointer in [
        "/data/fail_reason",
        "/fail_reason",
        "/data/error/message",
        "/error/message",
        "/data/result_url",
        "/message",
    ] {
        if let Some(text) = v
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            return text.chars().take(500).collect();
        }
    }
    "New API 视频任务失败，后端未返回错误详情".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            prompt: "生成一段视频".into(),
            negative_prompt: Some("低清晰度".into()),
            width: 720,
            height: 1280,
            duration_secs: 5,
            fps: 24,
            model: "grok-imagine-video".into(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: None,
        }
    }

    #[test]
    fn builds_newapi_request_body() {
        let body = NewApiVideoProvider::request_body(&request()).expect("valid body");

        assert_eq!(body["model"], "grok-imagine-video");
        assert_eq!(body["prompt"], "生成一段视频");
        assert_eq!(body["duration"], 5);
        assert_eq!(body["size"], "720x1280");
        assert!(body.get("image").is_none());
        assert!(body.get("duration_secs").is_none());
        assert!(body.get("reference_image_urls").is_none());
        assert!(body.get("width").is_none());
        assert!(body.get("height").is_none());
        assert!(body.get("fps").is_none());
    }

    #[test]
    fn rejects_material_reference_images_instead_of_dropping_them() {
        let mut req = request();
        req.reference_image_urls
            .push("data:image/png;base64,def".into());

        let err = NewApiVideoProvider::request_body(&req)
            .expect_err("newapi must reject unsupported reference images");

        assert!(err.to_string().contains("不支持素材参考图输入"));
        assert!(err.to_string().contains("当前收到 1 张参考图"));
    }

    #[test]
    fn non_veo_rejects_tail_frame_instead_of_dropping_it() {
        let mut req = request();
        req.first_frame_image_url = Some("data:image/png;base64,start".into());
        req.last_frame_image_url = Some("data:image/png;base64,end".into());

        let err = NewApiVideoProvider::request_body(&req)
            .expect_err("newapi must reject unsupported tail frame");

        assert!(err.to_string().contains("不支持尾帧输入"));
    }

    #[test]
    fn non_veo_uses_first_frame_image() {
        let mut req = request();
        req.first_frame_image_url = Some("data:image/png;base64,start".into());

        let body = NewApiVideoProvider::request_body(&req).expect("valid body");

        assert_eq!(body["image"], "data:image/png;base64,start");
    }

    #[test]
    fn builds_veo_first_frame_request_body() {
        let mut req = request();
        req.model = "veo-3.1-generate-preview".into();
        req.width = 1280;
        req.height = 720;
        req.first_frame_image_url = Some("data:image/png;base64,start".into());

        let body = NewApiVideoProvider::request_body(&req).expect("valid body");

        assert_eq!(body["model"], "veo-3.1-generate-preview");
        assert_eq!(body["duration"], 5);
        assert_eq!(body["duration_seconds"], 5);
        assert_eq!(body["number_of_videos"], 1);
        assert_eq!(body["enhance_prompt"], true);
        assert_eq!(body["size"], "1280x720");
        assert_eq!(body["aspect_ratio"], "16:9");
        assert_eq!(body["metadata"]["durationSeconds"], 5);
        assert_eq!(body["metadata"]["aspectRatio"], "16:9");
        assert_eq!(body["metadata"]["resolution"], "720p");
        assert_eq!(body["metadata"]["negativePrompt"], "低清晰度");
        assert_eq!(body["image"], "data:image/png;base64,start");
        assert!(body.get("last_frame").is_none());
        assert!(body.get("last_frame_image_url").is_none());
        assert!(body.get("reference_images").is_none());
    }

    #[test]
    fn rejects_veo_boundary_frames_instead_of_dropping_tail_frame() {
        let mut req = request();
        req.model = "veo-3.1-generate-preview".into();
        req.first_frame_image_url = Some("data:image/png;base64,start".into());
        req.last_frame_image_url = Some("data:image/png;base64,end".into());

        let err = NewApiVideoProvider::request_body(&req)
            .expect_err("newapi must reject unsupported tail frame");

        assert!(err.to_string().contains("不支持尾帧输入"));
    }

    #[test]
    fn rejects_veo_material_references_when_no_first_frame_is_available() {
        let mut req = request();
        req.model = "veo-3.1-generate-preview".into();
        req.reference_image_urls = vec![
            "data:image/png;base64,asset1".into(),
            "data:image/png;base64,asset2".into(),
        ];

        let err = NewApiVideoProvider::request_body(&req)
            .expect_err("newapi must reject unsupported reference images");

        assert!(err.to_string().contains("不支持素材参考图输入"));
        assert!(err.to_string().contains("当前收到 2 张参考图"));
    }

    #[test]
    fn extracts_task_id_from_top_level_or_data() {
        assert_eq!(
            extract_task_id(&json!({"task_id": "task-1"})).unwrap(),
            Some("task-1".into())
        );
        assert_eq!(
            extract_task_id(&json!({"data": {"id": "task-2"}})).unwrap(),
            Some("task-2".into())
        );
    }

    #[test]
    fn prefers_nested_done_status_over_outer_failure() {
        let value = json!({
            "data": {
                "status": "FAILURE",
                "fail_reason": "upstream returned unrecognized message",
                "data": {
                    "status": "done",
                    "video": {
                        "url": "https://vidgen.x.ai/video.mp4"
                    }
                }
            }
        });

        assert_eq!(extract_status(&value), Some("done"));
        let payload = find_video_url(&value).unwrap().expect("has video url");
        assert_eq!(payload.url, "https://vidgen.x.ai/video.mp4");
        assert_eq!(payload.format, None);
    }

    #[test]
    fn extracts_official_completed_response() {
        let value = json!({
            "task_id": "abcd",
            "status": "completed",
            "url": "https://example.com/result.webm",
            "format": "webm"
        });

        assert_eq!(extract_status(&value), Some("completed"));
        let payload = find_video_url(&value).unwrap().expect("has video url");
        assert_eq!(payload.url, "https://example.com/result.webm");
        assert_eq!(payload.format, Some("webm"));
    }
}
