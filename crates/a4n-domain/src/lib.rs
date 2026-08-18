//! Pure domain value objects.
//!
//! This crate must stay free of database, provider, sidecar transport, and UI
//! dependencies.

pub mod admin;
pub mod auth;
pub mod canvas;
pub mod character;
pub mod collaboration;
pub mod embedding;
pub mod galgame;
pub mod image;
pub mod import_review;
pub mod llm;
pub mod manga;
pub mod model_discovery;
pub mod profile_material;
pub mod progress;
pub mod project;
pub mod shooting_script;
pub mod source_sync;

pub use canvas::{
    CanvasFrameKind, CanvasNodeRefKind, GeneratedMediaReference, GeneratedMediaReferenceParseError,
    canvas_cache_key, canvas_transfer_runtime_identity_key, clear_canvas_transfer_runtime_identity,
    parse_canvas_cache_key, parse_generated_media_reference,
};
pub use llm::{
    DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, LLM_MAX_TOKEN_LIMIT,
    LLM_RESERVED_REQUEST_OPTION_KEYS, LlmMaxTokensConfig, llm_token_profiles,
};
pub use model_discovery::{DiscoveredModel, ModelDiscoveryResult};

pub const EXTERNAL_IMPORT_MODEL: &str = "external-import";
pub const VIDEO_TAIL_FRAME_OFFSET_SECS: f32 = 0.08;

use std::{collections::BTreeMap, fmt, str::FromStr};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    Dark,
}

pub type ThemeCssVariables = BTreeMap<String, String>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum VideoDurationMode {
    Range,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VideoAspectRatio {
    Ratio21x9,
    Ratio16x9,
    Ratio4x3,
    Ratio1x1,
    Ratio3x4,
    Ratio9x16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyPostprocessOperation {
    Insert,
    Delete,
    Replace,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DownstreamConflictStrategy {
    #[default]
    Reject,
    KeepDownstream,
    DeleteDownstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChapterId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowRunId(i64);

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("{field} 必须是正数")]
    NonPositiveId { field: &'static str },
    #[error("主题 mode 无效：{value}，只支持 light、dark")]
    InvalidThemeMode { value: String },
    #[error("视频 duration_mode 无效：{value}，只支持 range、fixed")]
    InvalidVideoDurationMode { value: String },
    #[error("视频画幅无效：{value}")]
    InvalidVideoAspectRatio { value: String },
    #[error("未知项目协作角色: {value}")]
    InvalidProjectMemberRole { value: String },
    #[error("项目访问范围无效: {value}")]
    InvalidProjectAccessScope { value: String },
    #[error("未知画布节点引用类型: {value}")]
    InvalidCanvasNodeRefKind { value: String },
    #[error("画布抽帧类型无效: {value}，只支持 first、tail、time")]
    InvalidCanvasFrameKind { value: String },
}

impl ThemeMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl fmt::Display for ThemeMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ThemeMode {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(DomainError::InvalidThemeMode {
                value: value.to_string(),
            }),
        }
    }
}

impl VideoDurationMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Range => "range",
            Self::Fixed => "fixed",
        }
    }
}

impl VideoAspectRatio {
    pub const ALL: [Self; 6] = [
        Self::Ratio21x9,
        Self::Ratio16x9,
        Self::Ratio4x3,
        Self::Ratio1x1,
        Self::Ratio3x4,
        Self::Ratio9x16,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ratio21x9 => "21:9",
            Self::Ratio16x9 => "16:9",
            Self::Ratio4x3 => "4:3",
            Self::Ratio1x1 => "1:1",
            Self::Ratio3x4 => "3:4",
            Self::Ratio9x16 => "9:16",
        }
    }

    pub const fn components(self) -> (i32, i32) {
        match self {
            Self::Ratio21x9 => (21, 9),
            Self::Ratio16x9 => (16, 9),
            Self::Ratio4x3 => (4, 3),
            Self::Ratio1x1 => (1, 1),
            Self::Ratio3x4 => (3, 4),
            Self::Ratio9x16 => (9, 16),
        }
    }

    pub fn supported_values() -> String {
        Self::ALL
            .iter()
            .map(|ratio| ratio.as_str())
            .collect::<Vec<_>>()
            .join("、")
    }
}

impl AssemblyPostprocessOperation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Insert => "insert",
            Self::Delete => "delete",
            Self::Replace => "replace",
        }
    }
}

impl fmt::Display for VideoDurationMode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for VideoDurationMode {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "range" => Ok(Self::Range),
            "fixed" => Ok(Self::Fixed),
            _ => Err(DomainError::InvalidVideoDurationMode {
                value: value.to_string(),
            }),
        }
    }
}

impl fmt::Display for VideoAspectRatio {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for VideoAspectRatio {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|ratio| ratio.as_str() == value)
            .ok_or_else(|| DomainError::InvalidVideoAspectRatio {
                value: value.to_string(),
            })
    }
}

impl ProjectId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "project_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl UserId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "user_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl ChapterId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "chapter_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl WorkflowRunId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "workflow_run_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

fn positive_id(value: i64, field: &'static str) -> Result<i64, DomainError> {
    if value <= 0 {
        return Err(DomainError::NonPositiveId { field });
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_mode_accepts_only_canonical_values() {
        assert_eq!("light".parse::<ThemeMode>().unwrap(), ThemeMode::Light);
        assert_eq!("dark".parse::<ThemeMode>().unwrap(), ThemeMode::Dark);
        assert!(" dark ".parse::<ThemeMode>().is_err());
        assert!("system".parse::<ThemeMode>().is_err());
    }

    #[test]
    fn video_duration_mode_accepts_only_canonical_values() {
        assert_eq!(
            "range".parse::<VideoDurationMode>().unwrap(),
            VideoDurationMode::Range
        );
        assert_eq!(
            "fixed".parse::<VideoDurationMode>().unwrap(),
            VideoDurationMode::Fixed
        );
        assert!(" fixed ".parse::<VideoDurationMode>().is_err());
        assert!("discrete".parse::<VideoDurationMode>().is_err());
    }

    #[test]
    fn video_aspect_ratio_accepts_only_canonical_values() {
        assert_eq!(
            "16:9".parse::<VideoAspectRatio>().unwrap(),
            VideoAspectRatio::Ratio16x9
        );
        assert_eq!(VideoAspectRatio::Ratio9x16.components(), (9, 16));
        assert!(" 16:9 ".parse::<VideoAspectRatio>().is_err());
        assert!("16/9".parse::<VideoAspectRatio>().is_err());
        assert!("2:1".parse::<VideoAspectRatio>().is_err());
    }
}
