use super::RagDataType;

pub(super) const fn as_str(value: RagDataType) -> &'static str {
    match value {
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

pub(super) const fn display_name(value: RagDataType) -> &'static str {
    match value {
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

pub(super) const fn retrieval_weight(value: RagDataType) -> f32 {
    match value {
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

pub(super) fn parse(input: &str) -> Option<RagDataType> {
    match input.trim().to_ascii_lowercase().as_str() {
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

pub(super) fn ingest_hint() -> String {
    RagDataType::INGESTABLE
        .iter()
        .map(|data_type| data_type.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}
