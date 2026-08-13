use std::{fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CanvasNodeRefKind {
    Inline,
    Config,
    CanvasTextAsset,
    PromptArtifact,
    MediaAsset,
    VideoSegment,
    AssemblyTimeline,
    GenerationSpec,
    GeneratedImage,
    GeneratedVideo,
}

impl CanvasNodeRefKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Config => "config",
            Self::CanvasTextAsset => "canvas_text_asset",
            Self::PromptArtifact => "prompt_artifact",
            Self::MediaAsset => "media_asset",
            Self::VideoSegment => "video_segment",
            Self::AssemblyTimeline => "assembly_timeline",
            Self::GenerationSpec => "generation_spec",
            Self::GeneratedImage => "generated_image",
            Self::GeneratedVideo => "generated_video",
        }
    }
}

impl fmt::Display for CanvasNodeRefKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CanvasNodeRefKind {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "inline" => Ok(Self::Inline),
            "config" => Ok(Self::Config),
            "canvas_text_asset" => Ok(Self::CanvasTextAsset),
            "prompt_artifact" => Ok(Self::PromptArtifact),
            "media_asset" => Ok(Self::MediaAsset),
            "video_segment" => Ok(Self::VideoSegment),
            "assembly_timeline" => Ok(Self::AssemblyTimeline),
            "generation_spec" => Ok(Self::GenerationSpec),
            "generated_image" => Ok(Self::GeneratedImage),
            "generated_video" => Ok(Self::GeneratedVideo),
            _ => Err(DomainError::InvalidCanvasNodeRefKind {
                value: value.to_string(),
            }),
        }
    }
}

const CANVAS_TRANSFER_RUNTIME_IDENTITY_KEYS: [&str; 3] =
    ["workflow_run_id", "provider_job_id", "config_id"];

pub fn clear_canvas_transfer_runtime_identity(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            for key in CANVAS_TRANSFER_RUNTIME_IDENTITY_KEYS {
                object.remove(key);
            }
            for value in object.values_mut() {
                clear_canvas_transfer_runtime_identity(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                clear_canvas_transfer_runtime_identity(value);
            }
        }
        _ => {}
    }
}

pub fn canvas_transfer_runtime_identity_key(value: &serde_json::Value) -> Option<&'static str> {
    match value {
        serde_json::Value::Object(object) => CANVAS_TRANSFER_RUNTIME_IDENTITY_KEYS
            .into_iter()
            .find(|key| object.contains_key(*key))
            .or_else(|| {
                object
                    .values()
                    .find_map(canvas_transfer_runtime_identity_key)
            }),
        serde_json::Value::Array(values) => {
            values.iter().find_map(canvas_transfer_runtime_identity_key)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CanvasNodeRefKind, canvas_transfer_runtime_identity_key,
        clear_canvas_transfer_runtime_identity,
    };
    use serde_json::json;

    #[test]
    fn canvas_node_ref_kind_accepts_only_canonical_values() {
        assert_eq!(
            "generated_image".parse::<CanvasNodeRefKind>().unwrap(),
            CanvasNodeRefKind::GeneratedImage
        );
        assert_eq!(
            "generated_video".parse::<CanvasNodeRefKind>().unwrap(),
            CanvasNodeRefKind::GeneratedVideo
        );
        assert!(
            "legacy_generated_image"
                .parse::<CanvasNodeRefKind>()
                .is_err()
        );
        assert!(" generated_video".parse::<CanvasNodeRefKind>().is_err());
    }

    #[test]
    fn canvas_transfer_runtime_identity_is_removed_recursively() {
        let mut value = json!({
            "config_id": 1,
            "canvas": {"workflow_run_id": 2, "node_id": 3},
            "metadata": [{"provider_job_id": "job", "generated_video_id": 4}]
        });

        assert_eq!(
            canvas_transfer_runtime_identity_key(&value),
            Some("config_id")
        );
        clear_canvas_transfer_runtime_identity(&mut value);

        assert_eq!(canvas_transfer_runtime_identity_key(&value), None);
        assert_eq!(value["canvas"]["node_id"], json!(3));
        assert_eq!(value["metadata"][0]["generated_video_id"], json!(4));
    }
}
