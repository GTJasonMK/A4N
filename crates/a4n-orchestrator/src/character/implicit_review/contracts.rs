use a4n_core::models::character::{protagonist_behavior_record, protagonist_profile};
use a4n_llm::provider::ChatRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize)]
pub struct GetImplicitReviewInput {
    pub profile_id: i64,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckImplicitUpdateInput {
    pub profile_id: i64,
    pub attribute_key: String,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyImplicitUpdateDecisionInput {
    pub decision: ImplicitUpdateDecisionSummary,
    pub chapter_number: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeEventSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub timestamp: Option<String>,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub change_description: String,
    pub old_value: Value,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionMarkSummary {
    pub id: i64,
    pub profile_id: i64,
    pub category: String,
    pub key: String,
    pub chapter_number: i32,
    pub reason: String,
    pub evidence: String,
    pub consecutive_count: i32,
    pub threshold: i32,
    pub last_marked_chapter: i32,
    pub is_executed: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyImplicitUpdateDecisionResultSummary {
    pub action: String,
    pub event: Option<AttributeEventSummary>,
    pub deletion_mark: Option<DeletionMarkSummary>,
    pub ready_for_deletion: bool,
    pub threshold: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitEvidenceSummary {
    pub behavior_id: i64,
    pub chapter_number: i32,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: Value,
    pub classification: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitAttributeReviewSummary {
    pub attribute_key: String,
    pub current_value: Value,
    pub total: usize,
    pub conform_count: usize,
    pub non_conform_count: usize,
    pub conform_rate: f64,
    pub threshold_reached: bool,
    pub evidence: Vec<ImplicitEvidenceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitReviewSummary {
    pub profile_id: i64,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
    pub attributes: Vec<ImplicitAttributeReviewSummary>,
    pub needs_review: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitUpdateDecisionSummary {
    pub profile_id: i64,
    pub attribute_key: String,
    pub current_value: Value,
    pub decision: String,
    pub reasoning: String,
    pub suggested_new_value: Option<Value>,
    pub evidence_summary: String,
    pub stats: ImplicitAttributeReviewSummary,
}

pub struct ImplicitUpdateReviewRequestPreview {
    pub request: ChatRequest,
}

pub struct CharacterSyncImplicitReviewInput<'a> {
    pub workflow_run_id: Option<i64>,
    pub profile: &'a protagonist_profile::Model,
    pub start_chapter: i32,
    pub final_attributes: &'a Value,
    pub behaviors: &'a [Value],
    pub deletion_candidates: &'a [Value],
}

pub(super) struct BehaviorForReview {
    pub(super) behavior_id: i64,
    pub(super) chapter_number: i32,
    pub(super) behavior_type: String,
    pub(super) behavior_description: String,
    pub(super) original_text: String,
    pub(super) behavior_tags: Value,
    pub(super) classification_results: Value,
    pub(super) created_at: Option<String>,
}

impl From<&protagonist_behavior_record::Model> for BehaviorForReview {
    fn from(value: &protagonist_behavior_record::Model) -> Self {
        Self {
            behavior_id: value.id,
            chapter_number: value.chapter_number,
            behavior_type: value.behavior_type.clone(),
            behavior_description: value.behavior_description.clone(),
            original_text: value.original_text.clone(),
            behavior_tags: value.behavior_tags.clone(),
            classification_results: value.classification_results.clone(),
            created_at: Some(value.created_at.to_string()),
        }
    }
}

#[derive(Debug)]
pub(super) struct ParsedImplicitDecision {
    pub(super) decision: String,
    pub(super) reasoning: String,
    pub(super) suggested_new_value: Option<Value>,
    pub(super) evidence_summary: String,
}

pub(super) struct CheckImplicitUpdateRequestContext {
    pub(super) profile_id: i64,
    pub(super) project_id: i64,
    pub(super) current_value: Value,
    pub(super) stats: ImplicitAttributeReviewSummary,
    pub(super) request: ChatRequest,
}

pub(super) struct ImplicitUpdateLlmRequestInput<'a> {
    pub(super) project_id: i64,
    pub(super) profile_id: i64,
    pub(super) profile_name: &'a str,
    pub(super) attribute_key: &'a str,
    pub(super) current_value: &'a Value,
    pub(super) stats: &'a ImplicitAttributeReviewSummary,
    pub(super) threshold: usize,
    pub(super) window_chapters: usize,
    pub(super) source_refs_json: Value,
    pub(super) call_key: String,
}
