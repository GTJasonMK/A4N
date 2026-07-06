use async_trait::async_trait;
use serde_json::{Map, Value, json};

use a4n_core::error::AppError;

use super::download::download_video_url;
use super::response::parse_json_response;
use super::types::{VideoGenerationRequest, VideoGenerationResult, VideoJobState, VideoProvider};

/// Seedance 2.0 provider for the documented `/task/submit` task API.
///
/// Reference media mode uses `functionMode = "omni_reference"` with
/// `image_files` / `video_files` / `audio_files`, referenced from the prompt as
/// `@image_file_N`, `@video_file_N`, and `@audio_file_N`.
pub struct Seedance2VideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
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

    fn task_url(&self, task_id: &str) -> String {
        if self.base_url.contains("{task_id}") {
            return self.base_url.replace("{task_id}", task_id);
        }
        if let Some(root) = self.base_url.strip_suffix("/task/submit") {
            return format!("{root}/task/{task_id}");
        }
        format!("{}/task/{task_id}", self.base_url)
    }

    fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        let media = req.reference_media.as_ref().ok_or_else(|| {
            AppError::InvalidArgument("Seedance 2.0 参考媒体模式缺少 reference_media".into())
        })?;
        if media.image_files.is_empty()
            && media.video_files.is_empty()
            && media.audio_files.is_empty()
        {
            return Err(AppError::InvalidArgument(
                "Seedance 2.0 参考媒体模式至少需要一个 image_files、video_files 或 audio_files"
                    .into(),
            ));
        }

        let mut parameters = Map::new();
        parameters.insert("prompt".into(), json!(req.prompt));
        parameters.insert("functionMode".into(), json!("omni_reference"));
        parameters.insert("image_files".into(), json!(media.image_files));
        parameters.insert("video_files".into(), json!(media.video_files));
        parameters.insert("audio_files".into(), json!(media.audio_files));
        parameters.insert(
            "ratio".into(),
            json!(ratio_from_size(req.width, req.height)),
        );
        if !(4..=15).contains(&req.duration_secs) {
            return Err(AppError::InvalidArgument(format!(
                "Seedance 2.0 duration 必须在 4-15 秒之间，当前为 {} 秒",
                req.duration_secs
            )));
        }
        parameters.insert("duration".into(), json!(req.duration_secs));

        Ok(json!({
            "model_id": req.model,
            "parameters": Value::Object(parameters),
        }))
    }

    pub fn request_body_preview(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        Self::request_body(req)
    }

    async fn send_json(&self, url: &str, body: Value) -> Result<Value, AppError> {
        let mut req = self.client.post(url).json(&body);
        if !self.api_key.trim().is_empty() {
            req = req.bearer_auth(self.api_key.trim());
        }
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

    async fn decode_completed(
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

fn ratio_from_size(width: i32, height: i32) -> &'static str {
    if width <= 0 || height <= 0 {
        return "16:9";
    }
    if width * 9 == height * 21 {
        return "21:9";
    }
    if width * 9 == height * 16 {
        return "16:9";
    }
    if width * 3 == height * 4 {
        return "4:3";
    }
    if width == height {
        return "1:1";
    }
    if width * 4 == height * 3 {
        return "3:4";
    }
    if width * 16 == height * 9 {
        return "9:16";
    }
    "16:9"
}

fn extract_task_id(v: &Value) -> Option<String> {
    text_at(v, &["task_id"])
        .or_else(|| text_at(v, &["id"]))
        .or_else(|| text_at(v, &["data", "task_id"]))
        .or_else(|| text_at(v, &["data", "id"]))
        .map(str::to_string)
}

fn extract_status(v: &Value) -> Option<&str> {
    text_at(v, &["status"]).or_else(|| text_at(v, &["data", "status"]))
}

fn result_video_url(v: &Value) -> Option<&str> {
    text_at(v, &["result", "video_url"])
        .or_else(|| text_at(v, &["data", "result", "video_url"]))
        .or_else(|| text_at(v, &["video_url"]))
}

fn failure_message(v: &Value) -> String {
    text_at(v, &["error", "message"])
        .or_else(|| text_at(v, &["data", "error", "message"]))
        .or_else(|| text_at(v, &["message"]))
        .unwrap_or("Seedance 2.0 视频任务失败")
        .to_string()
}

fn text_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

#[async_trait]
impl VideoProvider for Seedance2VideoProvider {
    async fn generate(
        &self,
        req: VideoGenerationRequest,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.base_url.is_empty() {
            return Err(AppError::ConfigError("视频配置缺少 API Base URL".into()));
        }
        let url = self.submit_url();
        let body = Self::request_body(&req)?;
        tracing::debug!("seedance2 video submit POST {url} model={}", req.model);
        let v = self.send_json(&url, body).await?;
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
}

#[cfg(test)]
mod tests {
    use super::super::types::VideoReferenceMediaRequest;
    use super::*;

    fn request() -> VideoGenerationRequest {
        VideoGenerationRequest {
            prompt: "@image_file_1 作为主角参考，跟随 @video_file_1 的镜头节奏，使用 @audio_file_1 的背景音乐".into(),
            negative_prompt: None,
            width: 1280,
            height: 720,
            duration_secs: 15,
            fps: 24,
            model: "st-ai/super-seed2".into(),
            first_frame_image_url: None,
            last_frame_image_url: None,
            reference_image_urls: Vec::new(),
            reference_media: Some(VideoReferenceMediaRequest {
                image_files: vec!["https://cdn.example.com/character.png".into()],
                video_files: vec!["https://cdn.example.com/camera.mp4".into()],
                audio_files: vec!["https://cdn.example.com/music.mp3".into()],
            }),
        }
    }

    #[test]
    fn builds_omni_reference_request_body() {
        let body = Seedance2VideoProvider::request_body(&request()).expect("valid body");
        assert_eq!(body["model_id"], "st-ai/super-seed2");
        assert_eq!(body["parameters"]["functionMode"], "omni_reference");
        assert_eq!(body["parameters"]["ratio"], "16:9");
        assert_eq!(body["parameters"]["duration"], 15);
        assert_eq!(
            body["parameters"]["image_files"][0],
            "https://cdn.example.com/character.png"
        );
        assert_eq!(
            body["parameters"]["video_files"][0],
            "https://cdn.example.com/camera.mp4"
        );
        assert_eq!(
            body["parameters"]["audio_files"][0],
            "https://cdn.example.com/music.mp3"
        );
        assert!(
            body["parameters"]["prompt"]
                .as_str()
                .unwrap()
                .contains("@image_file_1")
        );
    }

    #[test]
    fn rejects_missing_reference_media() {
        let mut req = request();
        req.reference_media = None;
        let err = Seedance2VideoProvider::request_body(&req).expect_err("missing media");
        assert!(err.to_string().contains("缺少 reference_media"));
    }

    #[test]
    fn rejects_duration_outside_seedance_range_without_clamping() {
        let mut req = request();
        req.duration_secs = 16;

        let err = Seedance2VideoProvider::request_body(&req).expect_err("duration must fail");

        assert!(err.to_string().contains("4-15 秒"));
        assert!(err.to_string().contains("16 秒"));
    }

    #[test]
    fn builds_task_url_from_submit_url() {
        let provider = Seedance2VideoProvider::new("https://api.example.com/task/submit", "key");
        assert_eq!(
            provider.task_url("task-1"),
            "https://api.example.com/task/task-1"
        );
    }
}
