//! RAG 数据类型契约。

use serde::{Deserialize, Serialize};

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

impl RagDataType {
    pub const CHAPTER_DERIVED: [RagDataType; 5] = [
        RagDataType::Chapter,
        RagDataType::ChapterSummary,
        RagDataType::ChapterMetadata,
        RagDataType::KeyEvent,
        RagDataType::Foreshadowing,
    ];

    pub const INGESTABLE: [RagDataType; 15] = [
        RagDataType::Chapter,
        RagDataType::Foreshadowing,
        RagDataType::Outline,
        RagDataType::PartOutline,
        RagDataType::Blueprint,
        RagDataType::BlueprintMetadata,
        RagDataType::WorldSetting,
        RagDataType::Character,
        RagDataType::Protagonist,
        RagDataType::CharacterState,
        RagDataType::CharacterAttributeEvent,
        RagDataType::Relationship,
        RagDataType::SceneProfile,
        RagDataType::SceneOccurrence,
        RagDataType::SceneTransition,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            RagDataType::Chapter => "chapter",
            RagDataType::ChapterSummary => "chapter_summary",
            RagDataType::ChapterMetadata => "chapter_metadata",
            RagDataType::KeyEvent => "key_event",
            RagDataType::Foreshadowing => "foreshadowing",
            RagDataType::Outline => "outline",
            RagDataType::PartOutline => "part_outline",
            RagDataType::Blueprint => "blueprint",
            RagDataType::BlueprintMetadata => "blueprint_metadata",
            RagDataType::WorldSetting => "world_setting",
            RagDataType::Character => "character",
            RagDataType::Protagonist => "protagonist",
            RagDataType::CharacterState => "character_state",
            RagDataType::CharacterAttributeEvent => "character_attribute_event",
            RagDataType::Relationship => "relationship",
            RagDataType::SceneProfile => "scene_profile",
            RagDataType::SceneOccurrence => "scene_occurrence",
            RagDataType::SceneTransition => "scene_transition",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            RagDataType::Chapter => "章节正文",
            RagDataType::ChapterSummary => "章节摘要",
            RagDataType::ChapterMetadata => "章节元数据",
            RagDataType::KeyEvent => "关键事件",
            RagDataType::Foreshadowing => "伏笔",
            RagDataType::Outline => "大纲",
            RagDataType::PartOutline => "分部大纲",
            RagDataType::Blueprint => "蓝图",
            RagDataType::BlueprintMetadata => "蓝图元数据",
            RagDataType::WorldSetting => "世界观",
            RagDataType::Character => "角色",
            RagDataType::Protagonist => "主角档案",
            RagDataType::CharacterState => "角色状态",
            RagDataType::CharacterAttributeEvent => "角色属性变更",
            RagDataType::Relationship => "关系",
            RagDataType::SceneProfile => "场景档案",
            RagDataType::SceneOccurrence => "场景出场",
            RagDataType::SceneTransition => "场景位移",
        }
    }

    pub const fn retrieval_weight(self) -> f32 {
        match self {
            RagDataType::Blueprint => 1.0,
            RagDataType::BlueprintMetadata => 0.98,
            RagDataType::WorldSetting => 0.95,
            RagDataType::Character => 0.9,
            RagDataType::KeyEvent => 0.88,
            RagDataType::Chapter => 0.85,
            RagDataType::ChapterSummary => 0.8,
            RagDataType::Outline => 0.75,
            RagDataType::ChapterMetadata => 0.72,
            RagDataType::PartOutline => 0.7,
            RagDataType::Foreshadowing => 0.65,
            RagDataType::Relationship => 0.6,
            RagDataType::SceneProfile => 0.58,
            RagDataType::SceneOccurrence => 0.57,
            RagDataType::SceneTransition => 0.56,
            RagDataType::CharacterState => 0.55,
            RagDataType::Protagonist => 0.5,
            RagDataType::CharacterAttributeEvent => 0.48,
        }
    }

    pub fn parse(input: &str) -> Option<Self> {
        let normalized = input.trim().to_ascii_lowercase();
        match normalized.as_str() {
            "chapter" => Some(RagDataType::Chapter),
            "chapter_summary" => Some(RagDataType::ChapterSummary),
            "chapter_metadata" => Some(RagDataType::ChapterMetadata),
            "key_event" => Some(RagDataType::KeyEvent),
            "foreshadowing" => Some(RagDataType::Foreshadowing),
            "outline" => Some(RagDataType::Outline),
            "part_outline" => Some(RagDataType::PartOutline),
            "blueprint" => Some(RagDataType::Blueprint),
            "blueprint_metadata" => Some(RagDataType::BlueprintMetadata),
            "world_setting" => Some(RagDataType::WorldSetting),
            "character" => Some(RagDataType::Character),
            "protagonist" => Some(RagDataType::Protagonist),
            "character_state" => Some(RagDataType::CharacterState),
            "character_attribute_event" => Some(RagDataType::CharacterAttributeEvent),
            "relationship" => Some(RagDataType::Relationship),
            "scene_profile" => Some(RagDataType::SceneProfile),
            "scene_occurrence" => Some(RagDataType::SceneOccurrence),
            "scene_transition" => Some(RagDataType::SceneTransition),
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

#[cfg(test)]
mod tests {
    use super::RagDataType;

    #[test]
    fn parses_canonical_names() {
        assert_eq!(RagDataType::parse("chapter"), Some(RagDataType::Chapter));
        assert_eq!(
            RagDataType::parse("chapter_summary"),
            Some(RagDataType::ChapterSummary)
        );
        assert_eq!(
            RagDataType::parse("foreshadowing"),
            Some(RagDataType::Foreshadowing)
        );
        assert_eq!(RagDataType::parse("outline"), Some(RagDataType::Outline));
        assert_eq!(
            RagDataType::parse("relationship"),
            Some(RagDataType::Relationship)
        );
        assert_eq!(
            RagDataType::parse("part_outline"),
            Some(RagDataType::PartOutline)
        );
        assert_eq!(
            RagDataType::parse("world_setting"),
            Some(RagDataType::WorldSetting)
        );
        assert_eq!(
            RagDataType::parse("character_state"),
            Some(RagDataType::CharacterState)
        );
        assert_eq!(
            RagDataType::parse("character_attribute_event"),
            Some(RagDataType::CharacterAttributeEvent)
        );
        assert_eq!(
            RagDataType::parse("scene_profile"),
            Some(RagDataType::SceneProfile)
        );
        assert_eq!(
            RagDataType::parse("scene_occurrence"),
            Some(RagDataType::SceneOccurrence)
        );
        assert_eq!(
            RagDataType::parse("scene_transition"),
            Some(RagDataType::SceneTransition)
        );
    }

    #[test]
    fn ingest_hint_includes_foreshadowing() {
        assert!(
            RagDataType::ingest_hint()
                .split(", ")
                .any(|data_type| data_type == "foreshadowing")
        );
    }

    #[test]
    fn retrieval_weight_prioritizes_core_story_constraints() {
        assert_eq!(RagDataType::Blueprint.retrieval_weight(), 1.0);
        assert!(
            RagDataType::WorldSetting.retrieval_weight() > RagDataType::Chapter.retrieval_weight()
        );
        assert!(
            RagDataType::Character.retrieval_weight() > RagDataType::Protagonist.retrieval_weight()
        );
        assert!(
            RagDataType::KeyEvent.retrieval_weight()
                > RagDataType::Foreshadowing.retrieval_weight()
        );
        assert!(
            RagDataType::CharacterState.retrieval_weight()
                > RagDataType::CharacterAttributeEvent.retrieval_weight()
        );
        assert!(
            RagDataType::SceneProfile.retrieval_weight()
                > RagDataType::CharacterAttributeEvent.retrieval_weight()
        );
    }

    #[test]
    fn rejects_aliases() {
        for alias in [
            "chapters",
            "chapter_content",
            "summary",
            "metadata",
            "events",
            "chapter_outline",
            "part_outlines",
            "blueprint_meta",
            "world",
            "protagonist_profile",
            "character_states",
            "protagonist_change",
            "attribute_event",
            "relationships",
            "scene",
            "scenes",
            "scene_profiles",
            "locations",
        ] {
            assert_eq!(
                RagDataType::parse(alias),
                None,
                "alias should be rejected: {alias}"
            );
        }
    }
}
