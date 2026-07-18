use super::{ImportReviewSummary, UpdateImportStatusInput, access, projections};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn update_import_status(
    state: &AppState,
    user_id: i64,
    review_id: i64,
    input: UpdateImportStatusInput,
) -> Result<ImportReviewSummary, AppError> {
    access::verify_review_access(state, user_id, review_id).await?;
    let updated = state
        .orchestrator
        .update_import_review_status(review_id, &input.status)
        .await?;
    projections::import_review_summary(&updated)
}

pub async fn delete_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<(), AppError> {
    access::verify_review_access(state, user_id, review_id).await?;
    state.orchestrator.delete_import_review(review_id).await
}

pub async fn reject_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    access::verify_review_access(state, user_id, review_id).await?;
    let updated = state
        .orchestrator
        .update_import_review_status(review_id, "rejected")
        .await?;
    projections::import_review_summary(&updated)
}
