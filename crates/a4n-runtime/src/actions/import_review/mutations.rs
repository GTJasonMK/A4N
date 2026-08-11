use super::{ImportReviewIdInput, ImportReviewSummary, projections};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn delete_import_review(
    state: &AppState,
    user_id: i64,
    input: ImportReviewIdInput,
) -> Result<(), AppError> {
    state
        .orchestrator()
        .delete_import_review_for_user_validated(user_id, input.review_id)
        .await
}

pub async fn reject_import_review(
    state: &AppState,
    user_id: i64,
    input: ImportReviewIdInput,
) -> Result<ImportReviewSummary, AppError> {
    let updated = state
        .orchestrator()
        .reject_import_review_for_user_validated(user_id, input.review_id)
        .await?;
    projections::import_review_summary(&updated)
}
