use a4n_core::models::project::import_review_queue;
use a4n_domain::import_review::ImportReviewStatus;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct ImportReviewCreateInput {
    pub project_id: i64,
    pub filename: String,
    pub analysis_result: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedImportReview {
    pub review: import_review_queue::Model,
    pub chapter_id: i64,
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

pub fn ensure_pending_import_review(
    review: &import_review_queue::Model,
    operation: &str,
) -> Result<(), ImportReviewMutationError> {
    if review.review_status != ImportReviewStatus::Pending.as_str() {
        return Err(ImportReviewMutationError::InvalidState {
            current: review.review_status.clone(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}
