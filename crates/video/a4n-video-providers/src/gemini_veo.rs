mod media_input;
mod response;
#[cfg(test)]
mod tests;

use self::media_input::{image_inline_data, image_inline_data_preview};
use self::response::{
    aspect_ratio_from_size, find_video_b64, find_video_uri, operation_error_message,
    operation_is_cancelled, required_operation_name, veo_resolution_from_size,
};
use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Map, Value, json};

use a4n_domain::ProviderError as AppError;
use a4n_video_provider_contract::{
    VideoGenerationResult, VideoJobState, VideoJobStatus, VideoProvider,
};
use a4n_video_sidecar_contract::VideoGenerationRequest;

use crate::download::download_video_url_with_api_key;
use crate::format::detect_video_format_from_bytes;
use crate::transport::{self, ApiKeyAuth};

pub(crate) struct GeminiVeoVideoProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiVeoVideoProvider {
    pub(crate) fn new(base_url: &str, api_key: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.trim().to_string(),
            client: reqwest::Client::new(),
        }
    }

    fn api_base_url(&self) -> Result<String, AppError> {
        let base = self.base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err(AppError::Configuration(
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
            return Err(AppError::Configuration(
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

    pub(crate) fn request_body_preview(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        Self::request_body_with_image_mapper(req, |raw, label| {
            Ok(image_inline_data_preview(raw, label))
        })
    }

    fn request_body_with_image_mapper(
        req: &VideoGenerationRequest,
        image_value: fn(&str, &str) -> Result<Value, AppError>,
    ) -> Result<Value, AppError> {
        if req.media.last_frame().is_some() && req.media.first_frame().is_none() {
            return Err(AppError::InvalidArgument(
                "Gemini Veo lastFrame 必须和首帧 image 一起使用".into(),
            ));
        }
        if req.media.reference_images().len() > 3 {
            return Err(AppError::InvalidArgument(
                "Gemini Veo referenceImages 最多支持 3 张素材参考图".into(),
            ));
        }

        let mut instance = Map::new();
        instance.insert("prompt".into(), json!(req.prompt));
        if let Some(image) = req.media.first_frame() {
            instance.insert("image".into(), image_value(image, "首帧 image")?);
        }
        if let Some(image) = req.media.last_frame() {
            instance.insert("lastFrame".into(), image_value(image, "尾帧 lastFrame")?);
        }
        if !req.media.reference_images().is_empty() {
            let references = req
                .media
                .reference_images()
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

    async fn decode_completed(&self, v: &Value) -> Result<VideoGenerationResult, AppError> {
        if let Some(payload) = find_video_b64(v)? {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(payload)
                .map_err(|e| AppError::Internal(format!("Gemini Veo 视频 base64 解码失败：{e}")))?;
            let format = detect_video_format_from_bytes(&bytes)?;
            return Ok(VideoGenerationResult {
                bytes,
                format,
                provider_job_id: None,
                metadata: v.clone(),
            });
        }

        let Some(uri) = find_video_uri(v) else {
            return Err(AppError::Internal(
                "Gemini Veo 任务已完成，但响应缺少 generatedVideos[0].video.uri".to_string(),
            ));
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
        provider_idempotency_key_sha256: Option<&str>,
    ) -> Result<VideoGenerationResult, AppError> {
        if self.api_key.is_empty() {
            return Err(AppError::Configuration(
                "Gemini Veo 视频配置缺少 api_key".into(),
            ));
        }
        let url = self.generation_url(&req.model)?;
        let body = Self::request_body(&req)?;
        tracing::debug!(
            "gemini veo video POST {url} model={} boundary_first={} boundary_last={} references={}",
            req.model,
            req.media.first_frame().is_some(),
            req.media.last_frame().is_some(),
            req.media.reference_images().len()
        );
        let v = transport::post_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedHeader("x-goog-api-key"),
            &url,
            &body,
            provider_idempotency_key_sha256,
            "Gemini Veo 视频生成",
        )
        .await?;
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
            return Err(AppError::Configuration(
                "Gemini Veo 视频配置缺少 api_key".into(),
            ));
        }
        let url = self.operation_url(job_id)?;
        let v = transport::get_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedHeader("x-goog-api-key"),
            &url,
            "Gemini Veo 任务查询",
        )
        .await?;
        if let Some(message) = operation_error_message(&v) {
            return Ok(VideoJobState::Failed(message));
        }
        if !response::operation_done(&v)? {
            return Ok(VideoJobState::Running);
        }
        Ok(VideoJobState::Completed(self.decode_completed(&v).await?))
    }

    async fn status(&self, job_id: &str) -> Result<VideoJobStatus, AppError> {
        if self.api_key.is_empty() {
            return Err(AppError::Configuration(
                "Gemini Veo 视频配置缺少 api_key".into(),
            ));
        }
        let value = transport::get_status_json(
            &self.client,
            &self.api_key,
            ApiKeyAuth::TrimmedHeader("x-goog-api-key"),
            &self.operation_url(job_id)?,
            "Gemini Veo 任务状态查询",
        )
        .await?;
        let Some(value) = value else {
            return Ok(VideoJobStatus::NotFound);
        };
        if operation_is_cancelled(&value) {
            return Ok(VideoJobStatus::Cancelled);
        }
        if let Some(message) = operation_error_message(&value) {
            return Ok(VideoJobStatus::Failed(message));
        }
        if response::operation_done(&value)? {
            return Ok(VideoJobStatus::Completed);
        }
        Ok(VideoJobStatus::Running)
    }
}
