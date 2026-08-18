pub use a4n_api_contract::character::{
    ApplyImplicitUpdateDecisionInput, CheckImplicitUpdateInput, GetImplicitReviewInput,
    ImplicitAttributeReviewSummary, ImplicitEvidenceSummary, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
use a4n_core::models::character::{protagonist_behavior_record, protagonist_profile};
use a4n_domain::character::ImplicitUpdateDecision;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

pub(crate) struct CharacterSyncImplicitReviewInput<'a> {
    pub workflow_run_id: i64,
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
        }
    }
}

#[derive(Debug)]
pub(super) struct ParsedImplicitDecision {
    pub(super) decision: ImplicitUpdateDecision,
    pub(super) reasoning: String,
    pub(super) suggested_new_value: Option<Value>,
    pub(super) evidence_summary: String,
}

pub(super) struct CheckImplicitUpdateRequestContext {
    pub(super) profile_id: i64,
    pub(super) project_id: i64,
    pub(super) stats: ImplicitAttributeReviewSummary,
    pub(super) call_key: String,
    pub(super) input_fingerprint_json: Value,
    pub(super) request: ChatRequest,
}

pub(super) struct PreparedImplicitUpdateRequest {
    pub(super) call_key: String,
    pub(super) request: ChatRequest,
    pub(super) input_fingerprint_json: Value,
}

pub(super) struct ImplicitUpdateLlmRequestInput<'a> {
    pub(super) project_id: i64,
    pub(super) profile_id: i64,
    pub(super) profile_name: &'a str,
    pub(super) attribute_key: &'a str,
    pub(super) stats: &'a ImplicitAttributeReviewSummary,
    pub(super) threshold: usize,
    pub(super) window_chapters: usize,
    pub(super) trigger: &'a str,
    pub(super) start_chapter: Option<i32>,
    pub(super) evidence_limit: usize,
}
