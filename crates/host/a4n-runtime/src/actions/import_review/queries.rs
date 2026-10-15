use super::{
    ImportProjectReviewsInput, ImportReviewIdInput, ImportReviewListItem, ImportReviewSummary,
};
use crate::AppState;
use a4n_error::AppError;

pub async fn list_import_queue(
    state: &AppState,
    user_id: i64,
    input: ImportProjectReviewsInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    state
        .import_review_read_application()
        .list(user_id, input)
        .await
}

pub async fn list_import_queue_index(
    state: &AppState,
    user_id: i64,
    input: ImportProjectReviewsInput,
) -> Result<Vec<ImportReviewListItem>, AppError> {
    state
        .import_review_read_application()
        .list_index(user_id, input)
        .await
}

pub async fn get_import_review(
    state: &AppState,
    user_id: i64,
    input: ImportReviewIdInput,
) -> Result<ImportReviewSummary, AppError> {
    state
        .import_review_read_application()
        .detail(user_id, input)
        .await
}
