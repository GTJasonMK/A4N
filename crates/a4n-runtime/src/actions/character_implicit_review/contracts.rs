use super::{ImplicitAttributeReviewSummary, ImplicitUpdateDecisionSummary};
use a4n_llm::provider::ChatRequest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct GetImplicitReviewInput {
    pub profile_id: i64,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct CheckImplicitUpdateInput {
    pub profile_id: i64,
    pub attribute_key: String,
    pub threshold: usize,
    pub window_chapters: usize,
    pub evidence_limit: usize,
}

#[derive(Debug, Deserialize)]
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

pub(super) struct CheckImplicitUpdateRequestContext {
    pub(super) project_id: i64,
    pub(super) current_value: Value,
    pub(super) stats: ImplicitAttributeReviewSummary,
    pub(super) request: ChatRequest,
}

pub(crate) struct ImplicitUpdateLlmRequestInput<'a> {
    pub(crate) project_id: i64,
    pub(crate) profile_id: Option<i64>,
    pub(crate) profile_name: &'a str,
    pub(crate) attribute_key: &'a str,
    pub(crate) current_value: &'a Value,
    pub(crate) stats: &'a ImplicitAttributeReviewSummary,
    pub(crate) threshold: usize,
    pub(crate) window_chapters: usize,
    pub(crate) source_refs_json: Option<Value>,
    pub(crate) call_key: Option<String>,
}
