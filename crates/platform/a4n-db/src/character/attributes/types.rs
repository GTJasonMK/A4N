use a4n_db_models::models::character::{
    attribute_event, protagonist_deletion_mark, protagonist_profile,
};
use serde_json::Value;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CharacterAttributeEventHistoryFilter {
    pub chapter_from: Option<i32>,
    pub chapter_to: Option<i32>,
    pub category: Option<String>,
    pub key: Option<String>,
    pub operation: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone)]
pub(in crate::character::attributes) struct CharacterAttributeEventInsertInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub source_kind: String,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub old_value: Value,
    pub new_value: Value,
    pub change_description: String,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone)]
pub struct CharacterAttributeMutationResult {
    pub profile: protagonist_profile::Model,
    pub event: attribute_event::Model,
}

#[derive(Debug, Clone)]
pub(in crate::character::attributes) struct CharacterDeletionMarkUpsertInput {
    pub profile_id: i64,
    pub source_kind: String,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppliedCharacterImplicitDecisionAction {
    Keep,
    Modify,
    MarkDeletion,
}

#[derive(Debug, Clone)]
pub struct AppliedCharacterImplicitDecisionRecord {
    pub profile: protagonist_profile::Model,
    pub action: AppliedCharacterImplicitDecisionAction,
    pub event: Option<attribute_event::Model>,
    pub deletion_mark: Option<protagonist_deletion_mark::Model>,
}

#[derive(Debug, Clone)]
pub struct CharacterAttributeDeletionExecutionResult {
    pub profile: protagonist_profile::Model,
    pub event: attribute_event::Model,
    pub consecutive_count: i32,
}

#[derive(Debug, Clone)]
pub(super) struct CharacterAttributeDeletionRecordInput {
    pub(super) profile_id: i64,
    pub(super) source_kind: String,
    pub(super) category: String,
    pub(super) key: String,
    pub(super) mark_count: i32,
    pub(super) event_cause: String,
    pub(super) evidence: String,
    pub(super) chapter_number: i32,
}

#[derive(Debug, Clone)]
pub(in crate::character::attributes) struct CharacterDeletionMarkExecutedUpdateInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub consecutive_count: i32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CharacterDeletionMarkRewindSummary {
    pub deleted: u64,
    pub rewound: u64,
}
