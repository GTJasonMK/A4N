use super::super::{
    CharacterAttributeDeletionRequestInput, CharacterAttributeValueChangeInput,
    CharacterDeletionMarkRewindSummary, CharacterImplicitReviewDecisionRecordInput,
    CharacterTimelineDeletionSummary, InsertBehaviorRecordInput,
};
use a4n_core::models::character::character_state_index;
use serde_json::Value;

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterSyncChapterStateRecordInput {
    pub chapter_number: i32,
    pub state_summary: String,
    pub current_location: Option<String>,
    pub current_goal: Option<String>,
    pub emotional_state: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CharacterSyncPersistenceInput {
    pub project_id: i64,
    pub profile_id: i64,
    pub profile_name: String,
    pub start_chapter: i32,
    pub target_chapter: i32,
    pub clear_to_chapter: Option<i32>,
    pub selected_chapter_numbers: Vec<i32>,
    pub baseline_state: Value,
    pub write_steps: Vec<CharacterSyncWriteStepRecordInput>,
    pub behaviors: Vec<Value>,
    pub classification_suggestions: Vec<Value>,
    pub deletion_candidates: Vec<Value>,
    pub chapter_states: Vec<CharacterSyncChapterStateRecordInput>,
    pub implicit_review_decisions: Value,
    pub generation_run: crate::generation::RecordCompletedGenerationRunInput,
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
