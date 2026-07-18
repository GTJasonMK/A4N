use a4n_core::models::project::import_review_queue;
use serde_json::Value;

pub const IMPORT_REVIEW_STATUS_PENDING: &str = "pending";
pub const IMPORT_REVIEW_STATUS_ANALYZING: &str = "analyzing";
pub const IMPORT_REVIEW_STATUS_REVIEWING: &str = "reviewing";
pub const IMPORT_REVIEW_STATUS_ACCEPTED: &str = "accepted";
pub const IMPORT_REVIEW_STATUS_REJECTED: &str = "rejected";
pub const IMPORT_REVIEW_STATUS_MODIFIED: &str = "modified";
pub const ALLOWED_IMPORT_REVIEW_STATUSES: [&str; 6] = [
    IMPORT_REVIEW_STATUS_PENDING,
    IMPORT_REVIEW_STATUS_ANALYZING,
    IMPORT_REVIEW_STATUS_REVIEWING,
    IMPORT_REVIEW_STATUS_ACCEPTED,
    IMPORT_REVIEW_STATUS_REJECTED,
    IMPORT_REVIEW_STATUS_MODIFIED,
];

#[derive(Debug, Clone, PartialEq)]
pub struct ImportReviewCreateInput {
    pub project_id: i64,
    pub filename: String,
    pub file_type: String,
    pub detected_encoding: Option<String>,
    pub parsed_chapters: i32,
    pub dedup_paragraphs: i32,
    pub dedup_rate: Option<f32>,
    pub analysis_result: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedImportReview {
    pub review: import_review_queue::Model,
    pub chapter_id: i64,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ModifyImportReviewInput {
    pub content: Option<String>,
    pub analysis_result: Option<Value>,
    pub ai_summary: Option<String>,
    pub ai_characters: Option<Value>,
    pub ai_world_setting: Option<Value>,
}

#[derive(Debug, thiserror::Error)]
pub enum ImportReviewMutationError {
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("状态无效：当前状态 {current}，不允许操作 {operation}")]
    InvalidState { current: String, operation: String },
}

pub fn normalize_import_review_status(
    status: &str,
) -> Result<&'static str, ImportReviewMutationError> {
    let status = status.trim();
    if status.is_empty() {
        return Err(ImportReviewMutationError::InvalidArgument(
            "导入审核状态不能为空".into(),
        ));
    }
    ALLOWED_IMPORT_REVIEW_STATUSES
        .iter()
        .copied()
        .find(|allowed| *allowed == status)
        .ok_or_else(|| {
            ImportReviewMutationError::InvalidArgument(format!(
                "不支持的导入审核状态: {status}；允许值: {}",
                ALLOWED_IMPORT_REVIEW_STATUSES.join(", ")
            ))
        })
}
