use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Map, Value, json};

use a4n_core::error::AppError;

use super::download::download_video_url_with_api_key;
use super::format::detect_video_format_from_bytes;
use super::types::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

/// Gemini Veo 原生 REST provider：
/// POST /v1beta/models/{model}:predictLongRunning
///
/// 首帧使用 `instances[0].image`，尾帧使用 `instances[0].lastFrame`。
pub struct GeminiVeoVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiVeoVideoProvider {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.trim().to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn api_base_url(&self) -> Result<String, AppError> {
        let base = self.base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err(AppError::ConfigError(
                "Gemini Veo 视频配置缺少 API Base URL".into(),
            ));
        }
        if base.ends_with("/v1beta") {
            Ok(base.to_string())
        } else if let Some(prefix) = base.strip_suffix("/v1") {
            Ok(format!("{prefix}/v1beta"))
        } else {
            Ok(format!("{base}/v1beta"))
        }
    }

    fn generation_url(&self, model: &str) -> Result<String, AppError> {
        let model = model.trim().trim_start_matches("models/").trim();
        if model.is_empty() {
            return Err(AppError::ConfigError(
                "Gemini Veo 视频配置缺少 model".into(),
            ));
        }
        Ok(format!(
            "{}/models/{}:predictLongRunning",
            self.api_base_url()?,
            model
        ))
    }

    fn operation_url(&self, operation_name: &str) -> Result<String, AppError> {
        let operation_name = operation_name.trim();
        if operation_name.is_empty() {
            return Err(AppError::InvalidArgument(
                "Gemini Veo operation name 不能为空".into(),
            ));
        }
        if operation_name.starts_with("http://") || operation_name.starts_with("https://") {
            return Ok(operation_name.to_string());
        }
        Ok(format!(
            "{}/{}",
            self.api_base_url()?,
            operation_name.trim_start_matches('/')
        ))
    }

    fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        Self::request_body_with_image_mapper(req, image_inline_data)
    }

    pub fn request_body_preview(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        Self::request_body_with_image_mapper(req, image_inline_data_preview)
    }

    fn request_body_with_image_mapper(
        req: &VideoGenerationRequest,
        image_value: fn(&str, &str) -> Result<Value, AppError>,
    ) -> Result<Value, AppError> {
        if req.last_frame_image_url.is_some() && req.first_frame_image_url.is_none() {
            return Err(AppError::InvalidArgument(
                "Gemini Veo lastFrame 必须和首帧 image 一起使用".into(),
            ));
        }
        if req.reference_image_urls.len() > 3 {
            return Err(AppError::InvalidArgument(
                "Gemini Veo referenceImages 最多支持 3 张素材参考图".into(),
            ));
        }

        let mut instance = Map::new();
        instance.insert("prompt".into(), json!(req.prompt));
        if let Some(image) = req.first_frame_image_url.as_deref() {
            instance.insert("image".into(), image_value(image, "首帧 image")?);
        }
        if let Some(image) = req.last_frame_image_url.as_deref() {
            instance.insert("lastFrame".into(), image_value(image, "尾帧 lastFrame")?);
        }
        if !req.reference_image_urls.is_empty() {
            let references = req
                .reference_image_urls
                .iter()
                .enumerate()
                .map(|(index, url)| {
                    Ok(json!({
                        "image": image_value(url, &format!("素材参考图 #{}", index + 1))?,
                        "referenceType": "asset",
                    }))
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            instance.insert("referenceImages".into(), json!(references));
        }

        let mut parameters = Map::new();
        parameters.insert("sampleCount".into(), json!(1));
        parameters.insert("durationSeconds".into(), json!(req.duration_secs));
        parameters.insert(
            "resolution".into(),
            json!(veo_resolution_from_size(req.width, req.height)),
        );
        if let Some(aspect_ratio) = aspect_ratio_from_size(req.width, req.height) {
            parameters.insert("aspectRatio".into(), json!(aspect_ratio));
        }
        if let Some(negative_prompt) = req
            .negative_prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            parameters.insert("negativePrompt".into(), json!(negative_prompt));
        }

        Ok(json!({
            "instances": [Value::Object(instance)],
            "parameters": Value::Object(parameters),
        }))
    }

    async fn send_json(&self, url: &str, body: Value) -> Result<Value, AppError> {
        let mut req = self.client.post(url).json(&body);
        if !self.api_key.is_empty() {
            req = req.header("x-goog-api-key", &self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Gemini Veo 视频生成请求失败：{e}")))?;
        parse_gemini_veo_json_response(resp, "Gemini Veo 视频生成").await
    }

    async fn get_json(&self, url: &str) -> Result<Value, AppError> {
        let mut req = self.client.get(url);
        if !self.api_key.is_empty() {
            req = req.header("x-goog-api-key", &self.api_key);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::Internal(anyhow::anyhow!("Gemini Veo 任务查询失败：{e}")))?;
        parse_gemini_veo_json_response(resp, "Gemini Veo 任务查询").await
    }

    async fn decode_completed(&self, v: &Value) -> Result<VideoGenerationResult, AppError> {
        if let Some(payload) = find_video_b64(v)? {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| {
                    AppError::Internal(anyhow::anyhow!("Gemini Veo 视频 base64 解码失败：{e}"))
                })?;
            let format = detect_video_format_from_bytes(&bytes)?;
            return Ok(VideoGenerationResult {
                bytes,
                format,
                provider_job_id: None,
                metadata: v.clone(),
            });
        }

        let Some(uri) = find_video_uri(v) else {
            return Err(AppError::Internal(anyhow::anyhow!(
                "Gemini Veo 任务已完成，但响应缺少 generatedVideos[0].video.uri"
            )));
        };
        let url = self.absolute_media_url(uri)?;
        let (bytes, format) =
            download_video_url_with_api_key(&self.client, &url, None, &self.api_key).await?;
        Ok(VideoGenerationResult {
            bytes,
            format,
            provider_job_id: None,
            metadata: v.clone(),
        })
    }

    fn absolute_media_url(&self, uri: &str) -> Result<String, AppError> {
        let uri = uri.trim();
        if uri.starts_with("http://") || uri.starts_with("https://") {
            return Ok(uri.to_string());
        }
        Ok(format!(
            "{}/{}",
            self.api_base_url()?,
            uri.trim_start_matches('/')
        ))
    }
}

#[async_trait]
impl VideoProvider for GeminiVeoVideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.api_key.is_empty() {
            return Err(AppError::ConfigError(
                "Gemini Veo 视频配置缺少 api_key".into(),
            ));
        }
        let url = self.generation_url(&req.model)?;
        let body = Self::request_body(&req)?;
        tracing::debug!(
            "gemini veo video POST {url} model={} boundary_first={} boundary_last={} references={}",
            req.model,
            req.first_frame_image_url.is_some(),
            req.last_frame_image_url.is_some(),
            req.reference_image_urls.len()
        );
        let v = self.send_json(&url, body).await?;
        let operation_name = required_operation_name(&v)?;
        Ok(VideoGenerationResult {
            bytes: Vec::new(),
            format: "mp4".into(),
            provider_job_id: Some(operation_name.to_string()),
            metadata: v,
        })
    }

    async fn poll(&self, job_id: &str) -> Result<VideoJobState, AppError> {
        if self.api_key.is_empty() {
            return Err(AppError::ConfigError(
                "Gemini Veo 视频配置缺少 api_key".into(),
            ));
        }
        let url = self.operation_url(job_id)?;
        let v = self.get_json(&url).await?;
        if let Some(message) = operation_error_message(&v) {
            return Ok(VideoJobState::Failed(message));
        }
        if v.get("done").and_then(Value::as_bool) != Some(true) {
            return Ok(VideoJobState::Running);
        }
        Ok(VideoJobState::Completed(self.decode_completed(&v).await?))
    }
}

async fn parse_gemini_veo_json_response(
    resp: reqwest::Response,
    label: &str,
) -> Result<Value, AppError> {
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("读取{label}响应失败：{e}")))?;
    if !status.is_success() {
        return Err(AppError::Internal(anyhow::anyhow!(
            "{label}返回 {status}：{}",
            text.chars().take(500).collect::<String>()
        )));
    }
    serde_json::from_str(&text)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("{label}响应解析失败：{e}")))
}

fn image_inline_data(raw: &str, label: &str) -> Result<Value, AppError> {
    let raw = raw.trim();
    let Some(rest) = raw.strip_prefix("data:") else {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} 必须是 data URL"
        )));
    };
    let Some((metadata, data)) = rest.split_once(',') else {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} data URL 缺少 base64 数据"
        )));
    };
    let mime_type = metadata
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| value.starts_with("image/"))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("Gemini Veo {label} 必须是 image/* data URL"))
        })?;
    let is_base64 = metadata
        .split(';')
        .skip(1)
        .any(|part| part.trim().eq_ignore_ascii_case("base64"));
    if !is_base64 {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} data URL 必须使用 base64 编码"
        )));
    }
    let data = data.trim();
    if data.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "Gemini Veo {label} base64 数据不能为空"
        )));
    }
    Ok(json!({
        "inlineData": {
            "mimeType": mime_type,
            "data": data,
        }
    }))
}

fn image_inline_data_preview(raw: &str, label: &str) -> Result<Value, AppError> {
    let raw = raw.trim();
    if let Some((mime_type, payload_chars)) = data_url_preview_parts(raw) {
        return Ok(json!({
            "inlineData": {
                "mimeType": mime_type,
                "data": format!("[已省略内联媒体数据: label={label}, payload_chars={payload_chars}]"),
            }
        }));
    }
    Ok(json!({
        "inlineData": {
            "mimeType": "image/png",
            "data": format!("[预览媒体占位: label={label}, source={}]", preview_media_source(raw)),
        }
    }))
}

fn data_url_preview_parts(raw: &str) -> Option<(&str, usize)> {
    let rest = raw.strip_prefix("data:")?;
    let (metadata, data) = rest.split_once(',')?;
    let mime_type = metadata
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| value.starts_with("image/"))?;
    Some((mime_type, data.trim().chars().count()))
}

fn preview_media_source(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() {
        return "<empty>".to_string();
    }
    let limit = 160;
    let prefix = raw.chars().take(limit).collect::<String>();
    if raw.chars().count() <= limit {
        prefix
    } else {
        format!("{prefix}...")
    }
}

fn required_operation_name(v: &Value) -> Result<&str, AppError> {
    v.get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            AppError::Internal(anyhow::anyhow!(
                "Gemini Veo 视频生成响应缺少 operation name"
            ))
        })
}

fn operation_error_message(v: &Value) -> Option<String> {
    let message = v
        .pointer("/error/message")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    Some(message.chars().take(500).collect())
}

fn find_video_uri(v: &Value) -> Option<&str> {
    for pointer in [
        "/response/generatedVideos/0/video/uri",
        "/response/generatedVideos/0/video/url",
        "/response/generateVideoResponse/generatedVideos/0/video/uri",
        "/response/generateVideoResponse/generatedVideos/0/video/url",
        "/response/generateVideoResponse/generatedSamples/0/video/uri",
        "/response/generateVideoResponse/generatedSamples/0/video/url",
        "/response/videos/0/uri",
        "/response/videos/0/url",
        "/response/video/uri",
        "/response/video/url",
    ] {
        if let Some(uri) = v
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(uri);
        }
    }
    None
}

fn find_video_b64(v: &Value) -> Result<Option<&str>, AppError> {
    for pointer in [
        "/response/generatedVideos/0/video/bytesBase64Encoded",
        "/response/generatedVideos/0/video/b64Json",
        "/response/generateVideoResponse/generatedVideos/0/video/bytesBase64Encoded",
        "/response/generateVideoResponse/generatedSamples/0/video/bytesBase64Encoded",
        "/response/videos/0/bytesBase64Encoded",
        "/response/bytesBase64Encoded",
    ] {
        if let Some(value) = v.pointer(pointer) {
            return value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(Some)
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!(
                        "Gemini Veo 响应字段 {pointer} 必须是非空字符串"
                    ))
                });
        }
    }
    Ok(None)
}

fn aspect_ratio_from_size(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width > height {
        Some("16:9")
    } else if height > width {
        Some("9:16")
    } else {
        Some("1:1")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            prompt: "生成一段视频".into(),
            negative_prompt: Some("低清晰度".into()),
            width: 1280,
            height: 720,
            duration_secs: 8,
            fps: 24,
            model: "veo-3.1-generate-preview".into(),
            first_frame_image_url: Some("data:image/png;base64,first".into()),
            last_frame_image_url: Some("data:image/png;base64,last".into()),
            reference_image_urls: vec!["data:image/jpeg;base64,asset".into()],
            reference_media: None,
        }
    }

    #[test]
    fn builds_native_veo_first_last_frame_body() {
        let body = GeminiVeoVideoProvider::request_body(&request()).expect("valid body");
        let instance = &body["instances"][0];

        assert_eq!(instance["prompt"], "生成一段视频");
        assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
        assert_eq!(instance["image"]["inlineData"]["data"], "first");
        assert_eq!(instance["lastFrame"]["inlineData"]["mimeType"], "image/png");
        assert_eq!(instance["lastFrame"]["inlineData"]["data"], "last");
        assert_eq!(instance["referenceImages"][0]["referenceType"], "asset");
        assert_eq!(
            instance["referenceImages"][0]["image"]["inlineData"]["mimeType"],
            "image/jpeg"
        );
        assert_eq!(body["parameters"]["sampleCount"], 1);
        assert_eq!(body["parameters"]["durationSeconds"], 8);
        assert_eq!(body["parameters"]["aspectRatio"], "16:9");
        assert_eq!(body["parameters"]["resolution"], "720p");
        assert_eq!(body["parameters"]["negativePrompt"], "低清晰度");
    }

    #[test]
    fn preview_body_redacts_inline_media_payloads() {
        let body = GeminiVeoVideoProvider::request_body_preview(&request()).expect("preview body");
        let instance = &body["instances"][0];

        assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
        assert_eq!(instance["lastFrame"]["inlineData"]["mimeType"], "image/png");
        assert!(
            instance["image"]["inlineData"]["data"]
                .as_str()
                .unwrap()
                .contains("已省略内联媒体数据")
        );
        assert_ne!(instance["image"]["inlineData"]["data"], "first");
        assert_ne!(instance["lastFrame"]["inlineData"]["data"], "last");
        assert_ne!(
            instance["referenceImages"][0]["image"]["inlineData"]["data"],
            "asset"
        );
    }

    #[test]
    fn preview_body_accepts_placeholder_media_urls_as_inline_data_shape() {
        let mut req = request();
        req.first_frame_image_url = Some("https://example.invalid/a4n-preview/first".into());
        req.last_frame_image_url = Some("https://example.invalid/a4n-preview/last".into());
        req.reference_image_urls = vec!["https://example.invalid/a4n-preview/ref".into()];

        let body = GeminiVeoVideoProvider::request_body_preview(&req).expect("preview body");
        let instance = &body["instances"][0];

        assert_eq!(instance["image"]["inlineData"]["mimeType"], "image/png");
        assert!(
            instance["image"]["inlineData"]["data"]
                .as_str()
                .unwrap()
                .contains("预览媒体占位")
        );
        assert_eq!(instance["referenceImages"][0]["referenceType"], "asset");
    }

    #[test]
    fn rejects_last_frame_without_first_frame() {
        let mut req = request();
        req.first_frame_image_url = None;

        let err = GeminiVeoVideoProvider::request_body(&req).expect_err("invalid body");

        assert!(err.to_string().contains("必须和首帧 image 一起使用"));
    }

    #[test]
    fn rejects_more_than_three_reference_images() {
        let mut req = request();
        req.reference_image_urls = vec![
            "data:image/png;base64,a".into(),
            "data:image/png;base64,b".into(),
            "data:image/png;base64,c".into(),
            "data:image/png;base64,d".into(),
        ];

        let err = GeminiVeoVideoProvider::request_body(&req).expect_err("too many refs");

        assert!(err.to_string().contains("最多支持 3 张"));
    }

    #[test]
    fn normalizes_base_url_variants() {
        assert_eq!(
            GeminiVeoVideoProvider::new("https://generativelanguage.googleapis.com", "key")
                .api_base_url()
                .unwrap(),
            "https://generativelanguage.googleapis.com/v1beta"
        );
        assert_eq!(
            GeminiVeoVideoProvider::new("https://example.com/v1", "key")
                .api_base_url()
                .unwrap(),
            "https://example.com/v1beta"
        );
        assert_eq!(
            GeminiVeoVideoProvider::new("https://example.com/v1beta", "key")
                .api_base_url()
                .unwrap(),
            "https://example.com/v1beta"
        );
    }

    #[test]
    fn extracts_operation_video_uri_shapes() {
        let value = json!({
            "done": true,
            "response": {
                "generateVideoResponse": {
                    "generatedSamples": [{
                        "video": { "uri": "https://example.com/out.mp4" }
                    }]
                }
            }
        });

        assert_eq!(find_video_uri(&value), Some("https://example.com/out.mp4"));
    }
}
