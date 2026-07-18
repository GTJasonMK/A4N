use super::RagDataType;

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
}
