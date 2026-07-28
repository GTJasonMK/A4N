use super::{
    BatchAcceptImportReviewsInput, ImportReviewSummary, ModifyImportReviewInput, projections,
};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn accept_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let accepted = state
        .import_review()
        .accept_import_review_for_user_validated(user_id, review_id, &host_port)
        .await?;
    projections::import_review_summary(&accepted.review)
}

pub async fn modify_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
    input: ModifyImportReviewInput,
) -> Result<ImportReviewSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    let accepted = state
        .import_review()
        .modify_import_review_for_user_validated(user_id, review_id, input, &host_port)
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
        .import_review()
        .batch_accept_import_reviews_for_user_validated(user_id, &input.review_ids, &host_port)
        .await?
        .iter()
        .map(|accepted| projections::import_review_summary(&accepted.review))
        .collect()
}
