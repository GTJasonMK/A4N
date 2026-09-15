//! Serialized contracts for the video provider sidecar.
//!
//! Provider implementations, HTTP clients, and Runtime adapters stay outside
//! this crate so a wire change does not pull the execution stack into clients.

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VIDEO_SERVICE_NAME: &str = "video";
pub const VIDEO_PROVIDER_CAPABILITY_VERSION: &str = "video-provider-v7";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum VideoMediaInput {
    None,
    Frames {
        first: String,
        last: Option<String>,
        references: Vec<String>,
    },
    References {
        images: Vec<String>,
        videos: Vec<String>,
        audio: Vec<String>,
    },
}

impl VideoMediaInput {
    pub fn from_images(
        first: Option<String>,
        last: Option<String>,
        references: Vec<String>,
    ) -> Result<Self, String> {
        match (first, last, references) {
            (Some(first), last, references) => Ok(Self::Frames {
                first,
                last,
                references,
            }),
            (None, Some(_), _) => Err("视频尾帧不能脱离首帧单独存在".to_string()),
            (None, None, images) if !images.is_empty() => Ok(Self::References {
                images,
                videos: Vec::new(),
                audio: Vec::new(),
            }),
            (None, None, _) => Ok(Self::None),
        }
    }

    pub fn first_frame(&self) -> Option<&str> {
        match self {
            Self::Frames { first, .. } => Some(first),
            _ => None,
        }
    }

    pub fn last_frame(&self) -> Option<&str> {
        match self {
            Self::Frames { last, .. } => last.as_deref(),
            _ => None,
        }
    }

    pub fn reference_images(&self) -> &[String] {
        match self {
            Self::Frames { references, .. } => references,
            Self::References { images, .. } => images,
            Self::None => &[],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoGenerationRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: i32,
    pub height: i32,
    pub duration_secs: i32,
    pub fps: i32,
    pub model: String,
    pub media: VideoMediaInput,
}

impl VideoGenerationRequest {
    pub fn validate_dimensions(&self) -> Result<(), String> {
        if self.width <= 0 || self.height <= 0 {
            return Err("视频请求宽度和高度必须为正整数".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoProviderPayload {
    pub backend: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_idempotency_key_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    pub request: VideoGenerationRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoPollPayload {
    pub backend: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<i64>,
    pub job_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoGenerationOutput {
    pub bytes_base64: String,
    pub format: String,
    pub provider_job_id: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VideoJobOutput {
    Pending,
    Running,
    Completed { result: VideoGenerationOutput },
    Failed { message: String },
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VideoStatusOutput {
    Pending,
    Running,
    Completed,
    Failed { message: String },
    Cancelled,
    NotFound,
}

impl<'de> Deserialize<'de> for VideoStatusOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut object = value
            .as_object()
            .cloned()
            .ok_or_else(|| D::Error::custom("video status output 必须是 JSON 对象"))?;
        let state = object
            .remove("state")
            .and_then(|value| value.as_str().map(str::to_string))
            .ok_or_else(|| D::Error::custom("video status output 缺少字符串 state"))?;
        let output = match state.as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => Self::Completed,
            "cancelled" => Self::Cancelled,
            "not_found" => Self::NotFound,
            "failed" => {
                let message = object
                    .remove("message")
                    .and_then(|value| value.as_str().map(str::to_string))
                    .ok_or_else(|| {
                        D::Error::custom("failed video status output 缺少字符串 message")
                    })?;
                Self::Failed { message }
            }
            other => {
                return Err(D::Error::custom(format!(
                    "未知 video status state: {other}"
                )));
            }
        };
        reject_unknown_fields(&object, "video status output", D::Error::custom)?;
        Ok(output)
    }
}

impl<'de> Deserialize<'de> for VideoJobOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let mut object = value
            .as_object()
            .cloned()
            .ok_or_else(|| D::Error::custom("video job output 必须是 JSON 对象"))?;
        let state = object
            .remove("state")
            .and_then(|value| value.as_str().map(str::to_string))
            .ok_or_else(|| D::Error::custom("video job output 缺少字符串 state"))?;
        let output = match state.as_str() {
            "pending" => Self::Pending,
            "running" => Self::Running,
            "completed" => {
                let result = object
                    .remove("result")
                    .ok_or_else(|| D::Error::custom("completed video job output 缺少 result"))?;
                Self::Completed {
                    result: serde_json::from_value(result).map_err(D::Error::custom)?,
                }
            }
            "failed" => {
                let message = object
                    .remove("message")
                    .and_then(|value| value.as_str().map(str::to_string))
                    .ok_or_else(|| {
                        D::Error::custom("failed video job output 缺少字符串 message")
                    })?;
                Self::Failed { message }
            }
            other => return Err(D::Error::custom(format!("未知 video job state: {other}"))),
        };
        reject_unknown_fields(&object, "video job output", D::Error::custom)?;
        Ok(output)
    }
}

fn reject_unknown_fields<E>(
    object: &serde_json::Map<String, Value>,
    context: &str,
    error: impl FnOnce(String) -> E,
) -> Result<(), E> {
    object.keys().next().map_or(Ok(()), |field| {
        Err(error(format!("{context} 包含未知字段 `{field}`")))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum VideoDeleteOutput {
    Deleted,
    NotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_input_rejects_tail_frame_without_first_frame() {
        let error = VideoMediaInput::from_images(None, Some("last".into()), Vec::new())
            .expect_err("a tail frame requires a first frame");
        assert!(error.contains("尾帧不能脱离首帧"));
    }

    #[test]
    fn sidecar_contract_rejects_unknown_fields() {
        let error = serde_json::from_value::<VideoProviderPayload>(serde_json::json!({
            "backend": "openai-compatible-video",
            "base_url": "https://example.invalid/v1",
            "request": {
                "prompt": "test",
                "negative_prompt": null,
                "width": 640,
                "height": 360,
                "duration_secs": 6,
                "fps": 24,
                "model": "video-model",
                "media": { "type": "none" }
            },
            "legacy_field": true
        }))
        .expect_err("unknown video payload fields must fail");
        assert!(error.to_string().contains("legacy_field"));
    }

    #[test]
    fn sidecar_outputs_reject_unknown_fields() {
        let error = serde_json::from_value::<VideoJobOutput>(serde_json::json!({
            "state": "pending",
            "legacy_status": true
        }))
        .expect_err("unknown video job fields must fail");
        assert!(error.to_string().contains("legacy_status"));

        let error = serde_json::from_value::<VideoStatusOutput>(serde_json::json!({
            "state": "completed",
            "result": {}
        }))
        .expect_err("status output must not contain a result");
        assert!(error.to_string().contains("result"));
    }
}
