use chrono::NaiveDateTime;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterAttributeEventRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub timestamp: NaiveDateTime,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub change_description: String,
    pub old_value: Value,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterBehaviorRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub behavior_description: String,
    pub behavior_type: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification_results: Value,
    pub related_characters: Option<Value>,
    pub impact_analysis: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterDeletionMarkRecord {
    pub id: i64,
    pub profile_id: i64,
    pub source_kind: String,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
    pub consecutive_count: i32,
    pub is_executed: bool,
    pub last_marked_chapter: i32,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterStateIndexRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub state_summary: String,
    pub key_attributes: Value,
    pub emotional_state: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSnapshotReferenceRecord {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub snapshot_hash: String,
    pub reference_count: i32,
    pub data: Value,
    pub created_at: NaiveDateTime,
}
