use super::{ImportProjectReviewsInput, ImportReviewSummary, projections};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn list_import_queue(
    state: &AppState,
    user_id: i64,
    input: ImportProjectReviewsInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    let reviews = state
        .orchestrator()
        .import_reviews_for_user_validated(user_id, input.project_id)
        .await?;
    reviews
        .iter()
        .map(projections::import_review_summary)
        .collect()
}
