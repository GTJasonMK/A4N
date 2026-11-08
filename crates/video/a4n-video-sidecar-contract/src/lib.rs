//! Serialized contracts for the video provider sidecar.
//!
//! Provider implementations, HTTP clients, and Runtime adapters stay outside
//! this crate so a wire change does not pull the execution stack into clients.

use a4n_generation_artifact_contract::{ArtifactLocation, ArtifactOwnership, ArtifactRef};
use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const VIDEO_SERVICE_NAME: &str = "video";
pub const VIDEO_PROVIDER_CAPABILITY_VERSION: &str = "video-provider-v8";
pub const VIDEO_SERVICE_ADDR_ENV: &str = "A4N_VIDEO_SERVICE_ADDR";
pub const VIDEO_SERVICE_DEFAULT_ADDR: &str = "127.0.0.1:47646";
pub const MAX_VIDEO_IMAGE_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_VIDEO_IMAGE_TOTAL_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum VideoMediaSource {
    Url { url: String },
    Artifact { artifact: ArtifactRef },
}

impl From<String> for VideoMediaSource {
    fn from(url: String) -> Self {
        Self::Url { url }
    }
}

impl From<&str> for VideoMediaSource {
    fn from(url: &str) -> Self {
        Self::from(url.to_string())
    }
}

pub type VideoGenerationInput = VideoGenerationRequest<VideoMediaSource>;

pub fn validate_video_image_artifact(artifact: &ArtifactRef) -> Result<&str, String> {
    if artifact.ownership != ArtifactOwnership::Host
        || artifact.artifact_kind != "generated_image"
        || artifact.owner_user_id.is_none_or(|id| id <= 0)
        || artifact.project_id.is_none_or(|id| id <= 0)
    {
        return Err("视频图片引用必须是带用户和项目范围的 Host 图片".into());
    }
    if artifact.artifact_id.is_empty()
        || artifact.artifact_id.len() > 128
        || !artifact
            .artifact_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        || artifact.sha256.len() != 64
        || !artifact
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        || artifact.byte_size == 0
        || artifact.byte_size > MAX_VIDEO_IMAGE_BYTES
    {
        return Err("视频图片引用的标识、SHA-256 或文件大小无效".into());
    }
    let ArtifactLocation::LocalPath(path) = &artifact.location else {
        return Err("视频图片引用必须使用本地媒体路径".into());
    };
    let prefix = format!("projects/{}/", artifact.project_id.unwrap());
    if !path.starts_with(&prefix)
        || path.contains('\\')
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err("视频图片路径必须位于引用项目的数据目录中".into());
    }
    Ok(path)
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum VideoMediaInput<Source = String> {
    None,
    Frames {
        first: Source,
        last: Option<Source>,
        references: Vec<Source>,
    },
    References {
        images: Vec<Source>,
        videos: Vec<String>,
        audio: Vec<String>,
    },
}

impl<Source> VideoMediaInput<Source> {
    pub fn from_images(
        first: Option<Source>,
        last: Option<Source>,
        references: Vec<Source>,
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

    pub fn first_frame(&self) -> Option<&Source> {
        match self {
            Self::Frames { first, .. } => Some(first),
            _ => None,
        }
    }

    pub fn last_frame(&self) -> Option<&Source> {
        match self {
            Self::Frames { last, .. } => last.as_ref(),
            _ => None,
        }
    }

    pub fn reference_images(&self) -> &[Source] {
        match self {
            Self::Frames { references, .. } => references,
            Self::References { images, .. } => images,
            Self::None => &[],
        }
    }

    pub fn try_map_images<T, E>(
        self,
        mut map: impl FnMut(Source) -> Result<T, E>,
    ) -> Result<VideoMediaInput<T>, E> {
        Ok(match self {
            Self::None => VideoMediaInput::None,
            Self::Frames {
                first,
                last,
                references,
            } => VideoMediaInput::Frames {
                first: map(first)?,
                last: last.map(&mut map).transpose()?,
                references: references
                    .into_iter()
                    .map(&mut map)
                    .collect::<Result<_, _>>()?,
            },
            Self::References {
                images,
                videos,
                audio,
            } => VideoMediaInput::References {
                images: images.into_iter().map(&mut map).collect::<Result<_, _>>()?,
                videos,
                audio,
            },
        })
    }
}

impl VideoMediaInput<VideoMediaSource> {
    pub fn validate_artifacts(&self) -> Result<(), String> {
        let mut total_bytes = 0_u64;
        for artifact in self.artifacts() {
            validate_video_image_artifact(artifact)?;
            total_bytes = total_bytes
                .checked_add(artifact.byte_size)
                .filter(|total| *total <= MAX_VIDEO_IMAGE_TOTAL_BYTES)
                .ok_or_else(|| "视频图片总大小超过限制".to_string())?;
        }
        Ok(())
    }

    pub fn artifacts(&self) -> impl Iterator<Item = &ArtifactRef> {
        let (first, last, images) = match self {
            Self::None => (None, None, &[][..]),
            Self::Frames {
                first,
                last,
                references,
            } => (Some(first), last.as_ref(), references.as_slice()),
            Self::References { images, .. } => (None, None, images.as_slice()),
        };
        first
            .into_iter()
            .chain(last)
            .chain(images)
            .filter_map(|source| match source {
                VideoMediaSource::Artifact { artifact } => Some(artifact),
                VideoMediaSource::Url { .. } => None,
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoGenerationRequest<Source = String> {
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub prompt: String,
    pub negative_prompt: Option<String>,
    #[schemars(range(min = 1))]
    pub width: i32,
    #[schemars(range(min = 1))]
    pub height: i32,
    #[schemars(range(min = 1))]
    pub duration_secs: i32,
    #[schemars(range(min = 1))]
    pub fps: i32,
    #[schemars(length(min = 1), regex(pattern = r"\S"))]
    pub model: String,
    pub media: VideoMediaInput<Source>,
}

impl<Source> VideoGenerationRequest<Source> {
    pub fn validate_dimensions(&self) -> Result<(), String> {
        if self.width <= 0 || self.height <= 0 {
            return Err("视频请求宽度和高度必须为正整数".to_string());
        }
        Ok(())
    }

    pub fn try_map_images<T, E>(
        self,
        map: impl FnMut(Source) -> Result<T, E>,
    ) -> Result<VideoGenerationRequest<T>, E> {
        Ok(VideoGenerationRequest {
            prompt: self.prompt,
            negative_prompt: self.negative_prompt,
            width: self.width,
            height: self.height,
            duration_secs: self.duration_secs,
            fps: self.fps,
            model: self.model,
            media: self.media.try_map_images(map)?,
        })
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
    pub request: VideoGenerationInput,
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
        let error = VideoMediaInput::<String>::from_images(None, Some("last".into()), Vec::new())
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
