use super::attributes::AttributeEventSummary;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListDeletionMarksInput {
    pub profile_id: i64,
    pub category: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestAttributeDeletionInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub reason: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestAttributeDeletionResultSummary {
    pub mark: DeletionMarkSummary,
    pub ready_for_deletion: bool,
    pub threshold: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteAttributeDeletionInput {
    pub profile_id: i64,
    pub chapter_number: Option<i32>,
    pub category: String,
    pub key: String,
    pub cause: Option<String>,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteAttributeDeletionResultSummary {
    pub event: AttributeEventSummary,
    pub consecutive_count: i32,
    pub threshold: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetDeletionMarksInput {
    pub profile_id: i64,
    pub category: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResetDeletionMarksResultSummary {
    pub deleted_marks: u64,
}
