use a4n_core::models::character::protagonist_behavior_record;
use serde::{Deserialize, Serialize};

pub const AUTO_IMPLICIT_REVIEW_THRESHOLD: usize = 5;
pub const AUTO_IMPLICIT_REVIEW_WINDOW_CHAPTERS: usize = 10;
pub const AUTO_IMPLICIT_REVIEW_EVIDENCE_LIMIT: usize = 10;
pub const MAX_IMPLICIT_REVIEW_WINDOW_CHAPTERS: usize = 50;
pub const MAX_IMPLICIT_REVIEW_EVIDENCE_LIMIT: usize = 50;

#[derive(Debug, Clone)]
pub struct BehaviorForReview {
    pub behavior_id: i64,
    pub chapter_number: i32,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: serde_json::Value,
    pub classification_results: serde_json::Value,
    pub created_at: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitEvidenceSummary {
    pub behavior_id: i64,
    pub chapter_number: i32,
    pub behavior_type: String,
    pub behavior_description: String,
    pub original_text: String,
    pub behavior_tags: serde_json::Value,
    pub classification: serde_json::Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplicitAttributeReviewSummary {
    pub attribute_key: String,
    pub current_value: serde_json::Value,
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
    pub current_value: serde_json::Value,
    pub decision: String,
    pub reasoning: String,
    pub suggested_new_value: Option<serde_json::Value>,
    pub evidence_summary: String,
    pub stats: ImplicitAttributeReviewSummary,
}

#[derive(Debug, Clone)]
pub struct ParsedImplicitDecision {
    pub decision: String,
    pub reasoning: String,
    pub suggested_new_value: Option<serde_json::Value>,
    pub evidence_summary: String,
}
