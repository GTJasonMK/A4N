//! Pure domain value objects.
//!
//! This crate must stay free of database, provider, sidecar transport, and UI
//! dependencies.

pub mod admin;
pub mod agent;
pub mod auth;
pub mod canvas;
pub mod chapter;
pub mod chapter_visual_plan;
pub mod character;
pub mod collaboration;
pub mod config;
pub mod cost;
pub mod embedding;
pub mod fact_feedback;
pub mod galgame;
pub mod generation;
pub mod hash;
pub mod image;
pub mod import_reverse;
pub mod import_review;
pub mod inspiration;
pub mod json_contract;
pub mod llm;
pub mod manga;
pub mod media;
pub mod media_authoring;
pub mod notifications;
pub mod object_profile;
pub mod package;
pub mod profile_material;
pub mod profile_ownership;
pub mod profile_text;
pub mod project;
pub mod project_reset;
pub mod prompt_language;
pub mod provider;
pub mod rag;
pub mod scene_profile;
pub mod shooting_script;
pub mod snapshot;
pub mod source_spans;
pub mod source_sync;
pub mod task_queue;
pub mod text_anchor;
pub mod text_location;
pub mod video;
pub mod video_prompt;
pub mod video_timeline;
pub mod video_transfer;
pub mod video_workflow;
pub mod workflow;

pub use canvas::{
    CanvasFrameKind, CanvasNodeRefKind, GeneratedMediaReference, GeneratedMediaReferenceParseError,
    canvas_cache_key, canvas_transfer_runtime_identity_key, clear_canvas_transfer_runtime_identity,
    parse_canvas_cache_key, parse_generated_media_reference,
};
pub use hash::source_text_hash;
pub use image::{DEFAULT_IMAGE_SIZES, ImageSize, default_image_sizes, validate_image_sizes};
pub use llm::{
    DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, LLM_RESERVED_REQUEST_OPTION_KEYS,
};
pub use media::{
    canonical_audio_mime, image_mime_from_path, media_mime_from_path, video_mime_from_path,
};
pub use provider::ProviderError;
pub use rag::{RAG_INDEX_FORMAT_VERSION, RAG_TEXT_INDEX_VERSION, RagDataType};

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

pub mod theme_edit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThemeAppearance {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeStreamDisplayMode {
    #[default]
    Fixed,
    Adaptive,
}

pub const THEME_STREAM_DISPLAY_MIN_RATE: u16 = 1;
pub const THEME_STREAM_DISPLAY_MAX_RATE: u16 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThemeStreamDisplaySettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_stream_display_rate")]
    pub rate: u16,
    #[serde(default)]
    pub mode: ThemeStreamDisplayMode,
}

impl Default for ThemeStreamDisplaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            rate: default_stream_display_rate(),
            mode: ThemeStreamDisplayMode::default(),
        }
    }
}

pub const THEME_DOCUMENT_VERSION: u16 = 1;
pub const THEME_GLOBAL_ROUTE_FAMILY: &str = "global";
pub const THEME_ROUTE_FAMILIES: &[&str] = &[
    THEME_GLOBAL_ROUTE_FAMILY,
    "home",
    "inspiration",
    "novel_detail",
    "writing_desk",
    "video_workspace",
    "video_studio",
    "comic_workspace",
    "galgame_workspace",
    "settings",
    "agents",
];

/// The only tokens that may be supplied by a user theme. Values are still
/// validated at the API boundary; this list prevents arbitrary CSS injection.
pub const THEME_TOKEN_NAMES: &[&str] = &[
    "--color-primary",
    "--color-primary-light",
    "--color-primary-dark",
    "--color-primary-pale",
    "--color-accent",
    "--color-accent-light",
    "--color-accent-dark",
    "--color-accent-pale",
    "--color-text-primary",
    "--color-text-secondary",
    "--color-text-tertiary",
    "--color-text-placeholder",
    "--color-text-disabled",
    "--color-bg-primary",
    "--color-bg-secondary",
    "--color-bg-tertiary",
    "--color-bg-card-hover",
    "--color-bg-muted",
    "--color-bg-accent",
    "--color-border",
    "--color-border-light",
    "--color-border-dark",
    "--status-success",
    "--status-success-bg",
    "--status-error",
    "--status-error-bg",
    "--status-warning",
    "--status-warning-bg",
    "--status-info",
    "--status-info-bg",
    "--color-button-text",
    "--color-button-text-secondary",
    "--shadow-card",
    "--shadow-card-hover",
    "--shadow-overlay",
    "--font-sans",
    "--font-serif",
    "--font-mono",
    "--pyqt-radius-xs",
    "--pyqt-radius-sm",
    "--pyqt-radius-md",
    "--pyqt-radius-lg",
    "--pyqt-radius-xl",
    "--a4n-workspace-opacity",
    "--a4n-panel-opacity",
    "--a4n-toolbar-opacity",
    "--a4n-card-opacity",
    "--a4n-control-opacity",
    "--a4n-dialog-opacity",
    "--a4n-surface-blur",
    "--a4n-shadow-strength",
];

const THEME_OPACITY_TOKEN_NAMES: &[&str] = &[
    "--a4n-workspace-opacity",
    "--a4n-panel-opacity",
    "--a4n-toolbar-opacity",
    "--a4n-card-opacity",
    "--a4n-control-opacity",
    "--a4n-dialog-opacity",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThemeBackgroundSettings {
    #[serde(default)]
    pub resource_key: Option<String>,
    #[serde(default = "default_background_opacity")]
    pub opacity: f32,
    #[serde(default = "default_background_overlay")]
    pub overlay: String,
    #[serde(default = "default_background_overlay_opacity")]
    pub overlay_opacity: f32,
    #[serde(default)]
    pub blur_px: f32,
    #[serde(default = "default_background_position")]
    pub position: String,
    #[serde(default = "default_background_size")]
    pub size: String,
}

impl Default for ThemeBackgroundSettings {
    fn default() -> Self {
        Self {
            resource_key: None,
            opacity: default_background_opacity(),
            overlay: default_background_overlay(),
            overlay_opacity: default_background_overlay_opacity(),
            blur_px: 0.0,
            position: default_background_position(),
            size: default_background_size(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ThemeDocument {
    pub version: u16,
    #[serde(default)]
    pub tokens: BTreeMap<String, String>,
    #[serde(default)]
    pub backgrounds: BTreeMap<String, ThemeBackgroundSettings>,
    #[serde(default)]
    pub stream_display: ThemeStreamDisplaySettings,
}

impl Default for ThemeDocument {
    fn default() -> Self {
        Self {
            version: THEME_DOCUMENT_VERSION,
            tokens: BTreeMap::new(),
            backgrounds: BTreeMap::new(),
            stream_display: ThemeStreamDisplaySettings::default(),
        }
    }
}

impl ThemeDocument {
    /// Export formats without bundled resources keep visual settings without
    /// leaving references to files that are not part of the export.
    pub fn without_resource_references(&self) -> Self {
        let mut document = self.clone();
        for background in document.backgrounds.values_mut() {
            background.resource_key = None;
        }
        document
    }
}

pub fn validate_theme_document(document: &ThemeDocument) -> Result<(), String> {
    if document.version != THEME_DOCUMENT_VERSION {
        return Err(format!(
            "主题文档版本不受支持：{}，当前只支持 {}",
            document.version, THEME_DOCUMENT_VERSION
        ));
    }
    if !(THEME_STREAM_DISPLAY_MIN_RATE..=THEME_STREAM_DISPLAY_MAX_RATE)
        .contains(&document.stream_display.rate)
    {
        return Err(format!(
            "流式显示速率必须在 {} 到 {} 字符/秒之间",
            THEME_STREAM_DISPLAY_MIN_RATE, THEME_STREAM_DISPLAY_MAX_RATE
        ));
    }
    if document.tokens.len() > 80 {
        return Err("主题高级 Token 数量不能超过 80 个".to_string());
    }
    for (name, value) in &document.tokens {
        if !THEME_TOKEN_NAMES.contains(&name.as_str()) {
            return Err(format!("主题 Token 不受支持：{name}"));
        }
        validate_theme_css_value(name, value)?;
    }
    if document.backgrounds.len() > THEME_ROUTE_FAMILIES.len() {
        return Err("主题背景配置数量超过允许范围".to_string());
    }
    for (route_family, background) in &document.backgrounds {
        if !THEME_ROUTE_FAMILIES.contains(&route_family.as_str()) {
            return Err(format!("主题路由族不受支持：{route_family}"));
        }
        if let Some(resource_key) = &background.resource_key
            && !THEME_ROUTE_FAMILIES.contains(&resource_key.as_str())
        {
            return Err(format!("主题资源标识无效：{resource_key}"));
        }
        validate_fraction("背景透明度", background.opacity)?;
        validate_fraction("背景遮罩透明度", background.overlay_opacity)?;
        if !background.blur_px.is_finite() || !(0.0..=40.0).contains(&background.blur_px) {
            return Err("背景模糊必须在 0 到 40 像素之间".to_string());
        }
        if !is_hex_color(&background.overlay) {
            return Err(format!("背景遮罩颜色无效：{}", background.overlay));
        }
        if !matches!(
            background.position.as_str(),
            "center" | "top" | "bottom" | "left" | "right"
        ) {
            return Err(format!("背景定位不受支持：{}", background.position));
        }
        if !matches!(background.size.as_str(), "cover" | "contain" | "auto") {
            return Err(format!("背景尺寸不受支持：{}", background.size));
        }
    }
    Ok(())
}

fn validate_theme_css_value(name: &str, value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 512 {
        return Err(format!("主题 Token {name} 的值为空或过长"));
    }
    let lower = value.to_ascii_lowercase();
    if [
        "url(",
        "expression(",
        "@import",
        "javascript:",
        "</style",
        "{",
        "}",
        ";",
    ]
    .iter()
    .any(|forbidden| lower.contains(forbidden))
    {
        return Err(format!("主题 Token {name} 包含不安全 CSS"));
    }
    if THEME_OPACITY_TOKEN_NAMES.contains(&name) {
        let opacity = value
            .parse::<f32>()
            .map_err(|_| format!("主题 Token {name} 必须是 0 到 1 之间的数字"))?;
        validate_fraction("主题组件透明度", opacity)?;
    }
    Ok(())
}

fn validate_fraction(label: &str, value: f32) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(format!("{label}必须在 0 到 1 之间"));
    }
    Ok(())
}

fn is_hex_color(value: &str) -> bool {
    let value = value.strip_prefix('#').unwrap_or_default();
    matches!(value.len(), 3 | 6 | 8) && value.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn default_background_opacity() -> f32 {
    0.22
}

fn default_background_overlay() -> String {
    "#000000".to_string()
}

fn default_background_overlay_opacity() -> f32 {
    0.16
}

fn default_background_position() -> String {
    "center".to_string()
}

fn default_background_size() -> String {
    "cover".to_string()
}

fn default_stream_display_rate() -> u16 {
    30
}

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
    #[error("主题外观偏好无效：{value}，只支持 system、light、dark")]
    InvalidThemeAppearance { value: String },
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

impl ThemeAppearance {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

impl fmt::Display for ThemeAppearance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for ThemeAppearance {
    type Err = DomainError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "system" => Ok(Self::System),
            "light" => Ok(Self::Light),
            "dark" => Ok(Self::Dark),
            _ => Err(DomainError::InvalidThemeAppearance {
                value: value.to_string(),
            }),
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
    fn theme_document_rejects_uncontrolled_css_and_invalid_backgrounds() {
        let mut document = ThemeDocument::default();
        document
            .tokens
            .insert("--unknown".to_string(), "red".to_string());
        assert!(validate_theme_document(&document).is_err());

        let mut document = ThemeDocument::default();
        document.tokens.insert(
            "--color-primary".to_string(),
            "url(https://example.com)".to_string(),
        );
        assert!(validate_theme_document(&document).is_err());

        document.tokens.clear();
        document
            .tokens
            .insert("--a4n-card-opacity".to_string(), "1.1".to_string());
        assert!(validate_theme_document(&document).is_err());

        document.backgrounds.insert(
            "home".to_string(),
            ThemeBackgroundSettings {
                opacity: 1.2,
                ..Default::default()
            },
        );
        assert!(validate_theme_document(&document).is_err());

        document.backgrounds.get_mut("home").unwrap().opacity = 0.5;
        document.backgrounds.get_mut("home").unwrap().resource_key = Some("unknown".to_string());
        assert!(validate_theme_document(&document).is_err());
    }

    #[test]
    fn definition_export_removes_unbundled_resource_references() {
        let mut document = ThemeDocument::default();
        document.backgrounds.insert(
            "home".to_string(),
            ThemeBackgroundSettings {
                resource_key: Some("home".to_string()),
                ..Default::default()
            },
        );

        let definition = document.without_resource_references();
        assert_eq!(definition.backgrounds["home"].resource_key, None);
    }

    #[test]
    fn theme_document_defaults_stream_display_and_validates_rate() {
        let mut document: ThemeDocument = serde_json::from_value(serde_json::json!({
            "version": 1,
            "tokens": {},
            "backgrounds": {}
        }))
        .expect("legacy theme document");
        assert_eq!(
            document.stream_display,
            ThemeStreamDisplaySettings::default()
        );
        assert!(!document.stream_display.enabled);

        document.stream_display.rate = 0;
        assert!(validate_theme_document(&document).is_err());
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
