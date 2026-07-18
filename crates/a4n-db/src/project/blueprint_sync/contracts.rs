use sea_orm::DbErr;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct BlueprintCharacterDraft {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,
    pub relationship_to_protagonist: Option<String>,
    pub arc_stages: Option<Value>,
}

pub const PART_OUTLINE_THRESHOLD: i32 = 50;
pub const CHAPTERS_PER_PART: i32 = 25;

pub fn needs_part_outlines_for_chapters(total_chapters: i32) -> bool {
    total_chapters > PART_OUTLINE_THRESHOLD
}

#[derive(Debug, Clone)]
pub struct BlueprintRelationshipDraft {
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: Option<String>,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BlueprintIncrementalSyncStats {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub deleted: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum BlueprintSyncError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
