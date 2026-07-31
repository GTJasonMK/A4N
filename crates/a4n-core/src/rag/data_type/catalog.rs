use super::RagDataType;

impl RagDataType {
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
