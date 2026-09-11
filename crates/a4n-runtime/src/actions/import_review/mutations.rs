use super::{ImportReviewIdInput, ImportReviewSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn delete_import_review(
    state: &AppState,
    user_id: i64,
    input: ImportReviewIdInput,
) -> Result<(), AppError> {
    state
        .import_review_application()
        .delete(user_id, input)
        .await
}

pub async fn reject_import_review(
    state: &AppState,
    user_id: i64,
    input: ImportReviewIdInput,
) -> Result<ImportReviewSummary, AppError> {
    state
        .import_review_application()
        .reject(user_id, input)
        .await
}
