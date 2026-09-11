use super::{ImportProjectReviewsInput, ImportReviewSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn list_import_queue(
    state: &AppState,
    user_id: i64,
    input: ImportProjectReviewsInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    state.import_review_application().list(user_id, input).await
}
