pub use a4n_orchestrator::import_review::ModifyImportReviewInput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReviewSummary {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub file_type: String,
    pub detected_encoding: Option<String>,
    pub parsed_chapters: i32,
    pub dedup_paragraphs: i32,
    pub dedup_rate: Option<f32>,
    pub analysis_result: Option<serde_json::Value>,
    pub review_status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchAcceptImportReviewsInput {
    pub review_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAnalysisStatusSummary {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupResult {
    pub content_hash: String,
    pub is_duplicate: bool,
    pub closest_distance: u32,
    pub matched_chapter_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateImportStatusInput {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckImportDedupInput {
    pub content: String,
    pub project_id: i64,
}
