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
