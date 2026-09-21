//! RAG 数据类型和索引格式契约。

use serde::{Deserialize, Serialize};

/// RAG 投影中可检索的记录类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RagDataType {
    Chapter,
    ChapterSummary,
    ChapterMetadata,
    KeyEvent,
    Foreshadowing,
    Outline,
    PartOutline,
    Blueprint,
    BlueprintMetadata,
    WorldSetting,
    Character,
    Protagonist,
    CharacterState,
    CharacterAttributeEvent,
    Relationship,
    SceneProfile,
    SceneOccurrence,
    SceneTransition,
}

pub const RAG_TEXT_INDEX_VERSION: &str = "ngram-v3";
pub const RAG_INDEX_FORMAT_VERSION: u32 = 1;

impl RagDataType {
    pub const INGESTABLE: [Self; 15] = [
        Self::Chapter,
        Self::Foreshadowing,
        Self::Outline,
        Self::PartOutline,
        Self::Blueprint,
        Self::BlueprintMetadata,
        Self::WorldSetting,
        Self::Character,
        Self::Protagonist,
        Self::CharacterState,
        Self::CharacterAttributeEvent,
        Self::Relationship,
        Self::SceneProfile,
        Self::SceneOccurrence,
        Self::SceneTransition,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Chapter => "chapter",
            Self::ChapterSummary => "chapter_summary",
            Self::ChapterMetadata => "chapter_metadata",
            Self::KeyEvent => "key_event",
            Self::Foreshadowing => "foreshadowing",
            Self::Outline => "outline",
            Self::PartOutline => "part_outline",
            Self::Blueprint => "blueprint",
            Self::BlueprintMetadata => "blueprint_metadata",
            Self::WorldSetting => "world_setting",
            Self::Character => "character",
            Self::Protagonist => "protagonist",
            Self::CharacterState => "character_state",
            Self::CharacterAttributeEvent => "character_attribute_event",
            Self::Relationship => "relationship",
            Self::SceneProfile => "scene_profile",
            Self::SceneOccurrence => "scene_occurrence",
            Self::SceneTransition => "scene_transition",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Chapter => "章节正文",
            Self::ChapterSummary => "章节摘要",
            Self::ChapterMetadata => "章节元数据",
            Self::KeyEvent => "关键事件",
            Self::Foreshadowing => "伏笔",
            Self::Outline => "大纲",
            Self::PartOutline => "分部大纲",
            Self::Blueprint => "蓝图",
            Self::BlueprintMetadata => "蓝图元数据",
            Self::WorldSetting => "世界观",
            Self::Character => "角色",
            Self::Protagonist => "主角档案",
            Self::CharacterState => "角色状态",
            Self::CharacterAttributeEvent => "角色属性变更",
            Self::Relationship => "关系",
            Self::SceneProfile => "场景档案",
            Self::SceneOccurrence => "场景出场",
            Self::SceneTransition => "场景位移",
        }
    }

    pub const fn retrieval_weight(self) -> f32 {
        match self {
            Self::Blueprint => 1.0,
            Self::BlueprintMetadata => 0.98,
            Self::WorldSetting => 0.95,
            Self::Character => 0.9,
            Self::KeyEvent => 0.88,
            Self::Chapter => 0.85,
            Self::ChapterSummary => 0.8,
            Self::Outline => 0.75,
            Self::ChapterMetadata => 0.72,
            Self::PartOutline => 0.7,
            Self::Foreshadowing => 0.65,
            Self::Relationship => 0.6,
            Self::SceneProfile => 0.58,
            Self::SceneOccurrence => 0.57,
            Self::SceneTransition => 0.56,
            Self::CharacterState => 0.55,
            Self::Protagonist => 0.5,
            Self::CharacterAttributeEvent => 0.48,
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "chapter" => Some(Self::Chapter),
            "chapter_summary" => Some(Self::ChapterSummary),
            "chapter_metadata" => Some(Self::ChapterMetadata),
            "key_event" => Some(Self::KeyEvent),
            "foreshadowing" => Some(Self::Foreshadowing),
            "outline" => Some(Self::Outline),
            "part_outline" => Some(Self::PartOutline),
            "blueprint" => Some(Self::Blueprint),
            "blueprint_metadata" => Some(Self::BlueprintMetadata),
            "world_setting" => Some(Self::WorldSetting),
            "character" => Some(Self::Character),
            "protagonist" => Some(Self::Protagonist),
            "character_state" => Some(Self::CharacterState),
            "character_attribute_event" => Some(Self::CharacterAttributeEvent),
            "relationship" => Some(Self::Relationship),
            "scene_profile" => Some(Self::SceneProfile),
            "scene_occurrence" => Some(Self::SceneOccurrence),
            "scene_transition" => Some(Self::SceneTransition),
            _ => None,
        }
    }

    pub fn ingest_hint() -> String {
        Self::INGESTABLE
            .iter()
            .map(|data_type| data_type.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl std::fmt::Display for RagDataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl AsRef<str> for RagDataType {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for RagDataType {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value).ok_or(())
    }
}
