use super::{
    AcceptImportReviewInput, BatchAcceptImportReviewsInput, ImportReviewSummary, projections,
};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn accept_import_review(
    state: &AppState,
    user_id: i64,
    input: AcceptImportReviewInput,
) -> Result<ImportReviewSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let accepted = state
        .orchestrator()
        .accept_import_review_for_user_validated(
            user_id,
            input.review_id,
            input.analysis_result,
            &host_port,
        )
        .await?;
    projections::import_review_summary(&accepted.review)
}

pub async fn batch_accept_import_reviews(
    state: &AppState,
    user_id: i64,
    input: BatchAcceptImportReviewsInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .orchestrator()
        .batch_accept_import_reviews_for_user_validated(user_id, &input.review_ids, &host_port)
        .await?
        .iter()
        .map(|accepted| projections::import_review_summary(&accepted.review))
        .collect()
}
