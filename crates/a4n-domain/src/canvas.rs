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

#[cfg(test)]
mod tests {
    use super::CanvasNodeRefKind;

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
            "deprecated_generated_image"
                .parse::<CanvasNodeRefKind>()
                .is_err()
        );
        assert!(" generated_video".parse::<CanvasNodeRefKind>().is_err());
    }
}
