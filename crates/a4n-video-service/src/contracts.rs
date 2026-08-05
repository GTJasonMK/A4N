use a4n_video::VideoGenerationRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VideoProviderPayload {
    pub(crate) backend: String,
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) provider_idempotency_key_sha256: Option<String>,
    #[serde(default)]
    pub(crate) project_id: Option<i64>,
    pub(crate) request: VideoGenerationRequest,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VideoPollPayload {
    pub(crate) backend: String,
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) project_id: Option<i64>,
    pub(crate) job_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VideoGenerationOutput {
    pub(crate) bytes_base64: String,
    pub(crate) format: String,
    pub(crate) provider_job_id: Option<String>,
    pub(crate) metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderProbeOutput {
    pub(crate) ok: bool,
    pub(crate) request_preview: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum VideoJobOutput {
    Pending,
    Running,
    Completed { result: VideoGenerationOutput },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum VideoStatusOutput {
    Pending,
    Running,
    Completed,
    Failed { message: String },
    Cancelled,
    NotFound,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub(crate) enum VideoDeleteOutput {
    Deleted,
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_provider_payload_accepts_project_scope() {
        let payload = serde_json::from_value::<VideoProviderPayload>(serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": "http://127.0.0.1:1",
            "project_id": 42,
            "request": {
                "prompt": "test prompt",
                "negative_prompt": null,
                "width": 640,
                "height": 360,
                "duration_secs": 6,
                "fps": 24,
                "model": "test-model",
                "media": { "type": "none" }
            }
        }))
        .expect("runtime video payload project scope must deserialize");

        assert_eq!(payload.project_id, Some(42));
    }

    #[test]
    fn video_payload_rejects_unknown_fields() {
        let error = serde_json::from_value::<VideoPollPayload>(serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": "http://127.0.0.1:1",
            "job_id": "job-1",
            "legacy_poll": true
        }))
        .expect_err("unknown video payload fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_poll`"));
    }

    #[test]
    fn video_provider_payload_rejects_legacy_parallel_media_fields() {
        let error = serde_json::from_value::<VideoProviderPayload>(serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": "http://127.0.0.1:1",
            "request": {
                "prompt": "test prompt",
                "negative_prompt": null,
                "width": 640,
                "height": 360,
                "duration_secs": 6,
                "fps": 24,
                "model": "test-model",
                "media": { "type": "none" },
                "first_frame_image_url": null
            }
        }))
        .expect_err("legacy parallel video media fields must fail");

        assert!(
            error
                .to_string()
                .contains("unknown field `first_frame_image_url`")
        );
    }
}
