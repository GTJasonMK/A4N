use super::super::CharacterDeletionMarkRewindSummary;
use a4n_core::models::{
    chapter::chapter,
    character::{attribute_event, character_state_index},
};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct CharacterConflictContextRows {
    pub chapters: Vec<chapter::Model>,
    pub state_timeline: Vec<character_state_index::Model>,
    pub attribute_events: Vec<attribute_event::Model>,
}

#[derive(Debug, Clone)]
pub struct CharacterStateIndexUpsertInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub state_summary: String,
    pub key_attributes: Value,
    pub emotional_state: Option<String>,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterTimelineDeletionSummary {
    pub attribute_event_ids: Vec<i64>,
    pub state_index_ids: Vec<i64>,
    pub deleted_attribute_events: u64,
    pub deleted_behaviors: u64,
    pub deleted_state_indexes: u64,
    pub deleted_snapshots: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterProfileRollbackResult {
    pub project_id: i64,
    pub deleted_timeline: CharacterTimelineDeletionSummary,
    pub deletion_mark_rewind: CharacterDeletionMarkRewindSummary,
    pub last_synced_chapter: i32,
}
