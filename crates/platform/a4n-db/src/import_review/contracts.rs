use a4n_db_models::models::project::import_review_queue;
use a4n_domain::import_review::{ImportReviewRecord, ImportReviewStatus};

#[derive(Debug, Clone, PartialEq)]
pub struct AcceptedImportReview {
    pub review: ImportReviewRecord,
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

impl From<crate::generation::GenerationLifecycleGateError> for ImportReviewMutationError {
    fn from(error: crate::generation::GenerationLifecycleGateError) -> Self {
        use crate::generation::GenerationLifecycleGateError as Error;
        match error {
            Error::InvalidArgument(message) => Self::InvalidArgument(message),
            Error::NotFound(message) => Self::NotFound(message),
            Error::Blocked(message) => Self::InvalidState {
                current: "project_lifecycle_gate".to_string(),
                operation: message,
            },
            Error::Corrupt(message) => Self::Database(sea_orm::DbErr::Custom(message)),
            Error::Database(error) => Self::Database(error),
        }
    }
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

pub(super) fn import_review_record(review: import_review_queue::Model) -> ImportReviewRecord {
    ImportReviewRecord {
        id: review.id,
        project_id: review.project_id,
        filename: review.filename,
        analysis_result: review.analysis_result,
        review_status: review.review_status,
        created_at: review.created_at,
        updated_at: review.updated_at,
    }
}
