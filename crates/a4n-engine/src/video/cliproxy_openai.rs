use async_trait::async_trait;
use reqwest::header::CONTENT_TYPE;
use serde_json::{Map, Value, json};

use a4n_core::error::AppError;

use super::download::download_video_url;
use super::format::detect_video_format_from_bytes;
use super::response::parse_json_response;
use super::types::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

const MAX_REFERENCE_IMAGES: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrokReferenceMode {
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

    fn content_url(&self, video_id: &str) -> String {
        format!("{}/content?variant=video", self.video_url(video_id))
    }

    fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
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

    async fn post_json(&self, url: &str, body: Value) -> Result<Value, AppError> {
        let mut request = self.client.post(url).json(&body);
        if !self.api_key.trim().is_empty() {
            request = request.bearer_auth(self.api_key.trim());
        }
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

    fn supports_openai_content_route(&self) -> bool {
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
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.videos_url();
        let body = self.request_body_for_config(&req)?;
        tracing::debug!("cliproxy video POST {url} model={}", req.model);
        let v = self.post_json(&url, body).await?;
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
}

fn request_body_with_mode(
    req: &VideoGenerationRequest,
    reference_mode: GrokReferenceMode,
) -> Result<Value, AppError> {
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("prompt".into(), json!(req.prompt));
    body.insert(
        "seconds".into(),
        json!(req.duration_secs.max(1).to_string()),
    );
    body.insert(
        "size".into(),
        json!(format!("{}x{}", req.width, req.height)),
    );
    if let Some(aspect_ratio) = aspect_ratio_from_size(req.width, req.height) {
        body.insert("aspect_ratio".into(), json!(aspect_ratio));
    }
    body.insert("resolution".into(), json!("720p"));

    let first_frame_image_url = req
        .first_frame_image_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty());
    let reference_image_urls = req
        .reference_image_urls
        .iter()
        .map(|url| url.trim())
        .filter(|url| !url.is_empty())
        .collect::<Vec<_>>();

    if reference_mode == GrokReferenceMode::FirstFrame {
        if let Some(last_frame_url) = req.last_frame_image_url.as_deref().map(str::trim)
            && !last_frame_url.is_empty()
        {
            return Err(AppError::InvalidArgument(
                "CLIProxy Grok 视频首帧模式只支持首帧 input_reference，不支持尾帧输入；请改用首帧串联模式或切换视频 provider"
                    .into(),
            ));
        }
        if let Some(image_url) =
            first_frame_image_url.or_else(|| reference_image_urls.first().copied())
        {
            ensure_http_media_url(image_url, "首帧图片 URL")?;
            body.insert(
                "input_reference".into(),
                json!({
                    "image_url": image_url,
                }),
            );
        }
        return Ok(Value::Object(body));
    }

    if first_frame_image_url.is_some()
        || req
            .last_frame_image_url
            .as_deref()
            .map(str::trim)
            .is_some_and(|url| !url.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "CLIProxy Grok 视频参考模式只读取 reference_image_urls；请将所有输入图片按实际参考顺序放入 reference_image_urls，不要设置 first_frame_image_url 或 last_frame_image_url"
                .into(),
        ));
    }

    if reference_image_urls.len() > MAX_REFERENCE_IMAGES {
        return Err(AppError::InvalidArgument(format!(
            "CLIProxy Grok 视频最多支持 {MAX_REFERENCE_IMAGES} 张素材参考图，当前 {} 张",
            reference_image_urls.len()
        )));
    }
    if !reference_image_urls.is_empty() && req.duration_secs > 10 {
        return Err(AppError::InvalidArgument(
            "CLIProxy Grok 视频使用素材参考图时最大时长为 10 秒，请降低时长或移除素材参考图".into(),
        ));
    }
    if !reference_image_urls.is_empty() && !grok_video_model_supports_reference_images(&req.model) {
        return Err(AppError::InvalidArgument(format!(
            "CLIProxy Grok 视频模型 {} 不支持 reference_images；请改用 grok-imagine-video，或切换为首帧模式 input_reference",
            req.model
        )));
    }
    let mut refs = Vec::with_capacity(reference_image_urls.len());
    for (index, url) in reference_image_urls.iter().enumerate() {
        ensure_http_media_url(url, &format!("素材参考图 #{}", index + 1))?;
        refs.push(json!({ "url": url }));
    }
    if !refs.is_empty() {
        body.insert("reference_images".into(), Value::Array(refs));
    }

    Ok(Value::Object(body))
}

fn grok_video_model_supports_reference_images(model: &str) -> bool {
    !grok_video_model_is_image_to_video_only(model)
}

fn grok_video_model_is_image_to_video_only(model: &str) -> bool {
    matches!(
        grok_video_model_base(model).as_deref(),
        Some("grok-imagine-video-1.5")
            | Some("grok-imagine-video-1.5-preview")
            | Some("grok-imagine-video-1.5-2026-05-30")
    )
}

fn grok_video_model_base(model: &str) -> Option<String> {
    let model = model.trim().to_ascii_lowercase();
    if model.is_empty() {
        return None;
    }
    Some(
        model
            .rsplit('/')
            .next()
            .unwrap_or(model.as_str())
            .trim()
            .to_string(),
    )
}

fn grok_reference_mode(value: Option<&Value>) -> Result<GrokReferenceMode, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(GrokReferenceMode::Reference);
    };
    let Some(mode) = value
        .get("grok_reference_mode")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|mode| !mode.is_empty())
    else {
        return Ok(GrokReferenceMode::Reference);
    };
    match mode {
        "reference" => Ok(GrokReferenceMode::Reference),
        "first_frame" => Ok(GrokReferenceMode::FirstFrame),
        other => Err(AppError::InvalidArgument(format!(
            "CLIProxy Grok 视频 grok_reference_mode 无效：{other}"
        ))),
    }
}

fn error_body_preview(value: &str) -> String {
    value.chars().take(500).collect::<String>()
}

fn completed_video_url(v: &Value) -> Option<&str> {
    for pointer in ["/video/url", "/video_url", "/url"] {
        if let Some(value) = v.pointer(pointer).and_then(Value::as_str).map(str::trim)
            && !value.is_empty()
        {
            return Some(value);
        }
    }
    None
}

fn video_id(v: &Value) -> Result<String, AppError> {
    for pointer in ["/id", "/request_id"] {
        if let Some(value) = v.pointer(pointer).and_then(Value::as_str).map(str::trim)
            && !value.is_empty()
        {
            return Ok(value.to_string());
        }
    }
    Err(AppError::Internal(anyhow::anyhow!(
        "CLIProxy 视频生成响应缺少 id"
    )))
}

fn status(v: &Value) -> Option<String> {
    v.pointer("/status")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

fn is_completed_status(status: Option<&str>) -> bool {
    matches!(
        status,
        Some("completed" | "complete" | "succeeded" | "success" | "done")
    )
}

fn failure_message(v: &Value) -> Option<String> {
    let status_failed = matches!(
        status(v).as_deref(),
        Some("failed" | "failure" | "error" | "expired" | "cancelled" | "canceled")
    );
    let message = v
        .pointer("/error/message")
        .and_then(Value::as_str)
        .or_else(|| v.pointer("/error").and_then(Value::as_str))
        .or_else(|| v.pointer("/message").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if status_failed {
        Some(message.unwrap_or_else(|| "视频生成失败".to_string()))
    } else {
        message.filter(|_| status_failed)
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
    if width * 3 == height * 4 {
        return Some("4:3");
    }
    if width * 4 == height * 3 {
        return Some("3:4");
    }
    if width * 2 == height * 3 {
        return Some("3:2");
    }
    if width * 3 == height * 2 {
        return Some("2:3");
    }
    None
}

fn ensure_http_media_url(url: &str, label: &str) -> Result<(), AppError> {
    if url.starts_with("http://") || url.starts_with("https://") {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "{label} 必须是 xAI 可访问的 http(s) 公网 URL，不能使用 base64/data URL"
    )))
}

fn format_from_content_type(content_type: &str) -> Option<String> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    match media_type.as_str() {
        "video/mp4" => Some("mp4".into()),
        "video/webm" => Some("webm".into()),
        "video/quicktime" => Some("mov".into()),
        "video/x-m4v" => Some("m4v".into()),
        _ => None,
    }
}

fn url_escape_path_segment(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            prompt: "animate".into(),
            negative_prompt: Some("noise".into()),
            width: 1280,
            height: 720,
            duration_secs: 5,
            fps: 24,
            model: "grok-imagine-video".into(),
            first_frame_image_url: Some("https://example.test/first.png".into()),
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: None,
        }
    }

    #[test]
    fn request_body_uses_openai_videos_shape_and_reference_images_by_default() {
        let mut req = request();
        req.first_frame_image_url = None;
        req.reference_image_urls = vec!["https://example.test/ref.png".into()];

        let body = CliProxyOpenAiVideoProvider::request_body(&req).unwrap();
        assert_eq!(body["model"], "grok-imagine-video");
        assert_eq!(body["seconds"], "5");
        assert_eq!(body["size"], "1280x720");
        assert_eq!(body["aspect_ratio"], "16:9");
        assert_eq!(
            body["reference_images"][0]["url"],
            "https://example.test/ref.png"
        );
        assert!(body.get("duration_secs").is_none());
        assert!(body.get("width").is_none());
        assert!(body.get("height").is_none());
        assert!(body.get("fps").is_none());
        assert!(body.get("last_frame_image_url").is_none());
        assert!(body.get("input_reference").is_none());
    }

    #[test]
    fn reference_mode_rejects_first_frame_slot_instead_of_promoting_it() {
        let req = request();
        let err = CliProxyOpenAiVideoProvider::request_body(&req).unwrap_err();

        assert!(
            err.to_string()
                .contains("参考模式只读取 reference_image_urls")
        );
    }

    #[test]
    fn first_frame_mode_rejects_tail_frame_instead_of_dropping_it() {
        let mut req = request();
        req.last_frame_image_url = Some("https://example.test/last.png".into());

        let err = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
            &req,
            Some(&json!({ "grok_reference_mode": "first_frame" })),
        )
        .unwrap_err();

        assert!(err.to_string().contains("不支持尾帧输入"));
    }

    #[test]
    fn first_frame_mode_rejects_inline_first_frame() {
        let mut req = request();
        req.first_frame_image_url = Some("data:image/png;base64,abc".into());
        let err = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
            &req,
            Some(&json!({ "grok_reference_mode": "first_frame" })),
        )
        .unwrap_err();
        assert!(err.to_string().contains("http(s)"));
    }

    #[test]
    fn request_body_sends_references_only_without_first_frame() {
        let mut req = request();
        req.first_frame_image_url = None;
        req.reference_image_urls = vec!["https://example.test/ref.png".into()];
        let body = CliProxyOpenAiVideoProvider::request_body(&req).unwrap();
        assert_eq!(
            body["reference_images"][0]["url"],
            "https://example.test/ref.png"
        );
        assert!(body.get("input_reference").is_none());
    }

    #[test]
    fn image_to_video_only_models_reject_reference_images_locally() {
        for model in [
            "grok-imagine-video-1.5",
            "grok-imagine-video-1.5-preview",
            "grok-imagine-video-1.5-2026-05-30",
        ] {
            let mut req = request();
            req.model = model.into();
            req.first_frame_image_url = None;
            req.reference_image_urls = vec!["https://example.test/ref.png".into()];

            let err = CliProxyOpenAiVideoProvider::request_body(&req).unwrap_err();

            assert!(err.to_string().contains("不支持 reference_images"));
            assert!(err.to_string().contains(model));
        }
    }

    #[test]
    fn first_frame_mode_uses_first_frame_input_reference() {
        let req = request();
        let body = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
            &req,
            Some(&json!({ "grok_reference_mode": "first_frame" })),
        )
        .unwrap();

        assert_eq!(
            body["input_reference"]["image_url"],
            "https://example.test/first.png"
        );
        assert!(body.get("reference_images").is_none());
    }

    #[test]
    fn first_frame_mode_promotes_first_reference_to_input_reference() {
        let mut req = request();
        req.first_frame_image_url = None;
        req.reference_image_urls = vec![
            "https://example.test/ref-1.png".into(),
            "https://example.test/ref-2.png".into(),
        ];

        let body = CliProxyOpenAiVideoProvider::request_body_preview_with_options(
            &req,
            Some(&json!({ "grok_reference_mode": "first_frame" })),
        )
        .unwrap();

        assert_eq!(
            body["input_reference"]["image_url"],
            "https://example.test/ref-1.png"
        );
        assert!(body.get("reference_images").is_none());
    }

    #[test]
    fn completed_video_url_supports_native_and_openai_wrapper_payloads() {
        assert_eq!(
            completed_video_url(&json!({
                "request_id": "job-1",
                "status": "done",
                "video": { "url": "https://cdn.example/video.mp4" }
            })),
            Some("https://cdn.example/video.mp4")
        );
        assert_eq!(
            completed_video_url(&json!({
                "id": "job-1",
                "status": "completed",
                "video_url": "https://cdn.example/openai-wrapper.mp4"
            })),
            Some("https://cdn.example/openai-wrapper.mp4")
        );
        assert_eq!(completed_video_url(&json!({ "status": "completed" })), None);
    }

    #[test]
    fn openai_content_route_is_only_used_for_openai_wrapper_base_url() {
        let native = CliProxyOpenAiVideoProvider::new("http://127.0.0.1:8317/v1", "key");
        assert!(!native.supports_openai_content_route());
        assert_eq!(
            native.content_url("job-1"),
            "http://127.0.0.1:8317/v1/videos/job-1/content?variant=video"
        );

        let wrapper = CliProxyOpenAiVideoProvider::new("http://127.0.0.1:8317/openai/v1", "key");
        assert!(wrapper.supports_openai_content_route());
        assert_eq!(
            wrapper.content_url("job-1"),
            "http://127.0.0.1:8317/openai/v1/videos/job-1/content?variant=video"
        );
    }
}
