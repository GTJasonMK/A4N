use a4n_domain::import_review::ImportReviewStatus;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ImportReviewSummary {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub analysis_result: Value,
    pub review_status: ImportReviewStatus,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportProjectReviewsInput {
    pub project_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ImportReviewIdInput {
    pub review_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AcceptImportReviewInput {
    pub review_id: i64,
    pub analysis_result: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BatchAcceptImportReviewsInput {
    pub review_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct DedupResult {
    pub content_hash: String,
    pub is_duplicate: bool,
    pub closest_distance: u32,
    pub matched_chapter_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CheckImportDedupInput {
    pub content: String,
    pub project_id: i64,
}
