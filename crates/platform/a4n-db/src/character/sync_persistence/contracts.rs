use super::super::{CharacterDeletionMarkRewindSummary, CharacterTimelineDeletionSummary};
use a4n_db_models::models::character::character_state_index;
use a4n_domain::character::{
    CharacterAttributeDeletionRequestInput, CharacterAttributeValueChangeInput,
    CharacterImplicitReviewDecisionRecordInput, CharacterSyncChapterStateRecordInput,
    InsertBehaviorRecordInput,
};
use a4n_domain::generation::RecordCompletedGenerationRunInput;
use chrono::NaiveDateTime;
use serde::Serialize;
use serde_json::Value;

use a4n_domain::chapter::ChapterSourceSnapshot;

#[derive(Debug, Clone)]
pub struct CharacterSyncAttributeEventRecordInput {
    pub operation: String,
    pub change: CharacterAttributeValueChangeInput,
    pub from_classification: bool,
}

#[derive(Debug, Clone)]
pub enum CharacterSyncWriteStepRecordInput {
    AttributeEvent(CharacterSyncAttributeEventRecordInput),
    Behavior(InsertBehaviorRecordInput),
    DeletionCandidate(CharacterAttributeDeletionRequestInput),
    ImplicitReview(CharacterImplicitReviewDecisionRecordInput),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterSyncWriteSummary {
    pub changes_applied: usize,
    pub classification_suggestions_applied: usize,
    pub behaviors_recorded: usize,
    pub deletions_marked: usize,
    pub deletions_executed: usize,
    pub implicit_reviews_applied: usize,
    pub attribute_event_ids: Vec<i64>,
    pub protected_attributes: Vec<CharacterSyncProtectedAttribute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CharacterSyncProtectedAttribute {
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub operation: String,
}

#[derive(Debug, Clone)]
pub struct CharacterSyncPersistenceInput {
    pub project_id: i64,
    pub profile_id: i64,
    pub profile_name: String,
    pub expected_profile_updated_at: NaiveDateTime,
    pub expected_profile_last_synced_chapter: i32,
    pub expected_profile_base_attributes: Value,
    pub expected_profile_current_state: Value,
    pub start_chapter: i32,
    pub target_chapter: i32,
    pub clear_to_chapter: Option<i32>,
    pub selected_chapter_numbers: Vec<i32>,
    pub source_chapters: Vec<ChapterSourceSnapshot>,
    pub baseline_state: Value,
    pub write_steps: Vec<CharacterSyncWriteStepRecordInput>,
    pub behaviors: Vec<Value>,
    pub classification_suggestions: Vec<Value>,
    pub deletion_candidates: Vec<Value>,
    pub chapter_states: Vec<CharacterSyncChapterStateRecordInput>,
    pub implicit_review_decisions: Value,
    pub generation_run: RecordCompletedGenerationRunInput,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppliedCharacterSyncPersistenceRecord {
    pub deleted_timeline: CharacterTimelineDeletionSummary,
    pub deletion_mark_rewind: CharacterDeletionMarkRewindSummary,
    pub write_summary: CharacterSyncWriteSummary,
    pub target_snapshot_hash: String,
    pub state_indexes: Vec<character_state_index::Model>,
}

#[derive(Debug, Clone, PartialEq)]
pub(in crate::character) struct CharacterSyncStoredSnapshotsRecord {
    pub(super) target_hash: String,
    pub(super) target_attributes: Value,
    pub(super) state_indexes: Vec<character_state_index::Model>,
}
