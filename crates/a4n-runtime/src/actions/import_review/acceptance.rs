use super::{
    BatchAcceptImportReviewsInput, ImportReviewSummary, ModifyImportReviewInput, access,
    projections,
};
use crate::AppState;
use a4n_core::error::AppError;

pub async fn accept_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    let review = access::verify_review_access(state, user_id, review_id).await?;
    let accepted = state.orchestrator.accept_import_review(review).await?;
    crate::workflow::postprocess_chapter_after_content_change(state, accepted.chapter_id).await?;
    projections::import_review_summary(&accepted.review)
}

pub async fn modify_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
    input: ModifyImportReviewInput,
) -> Result<ImportReviewSummary, AppError> {
    let review = access::verify_review_access(state, user_id, review_id).await?;
    let accepted = state
        .orchestrator
        .modify_import_review(review, input)
        .await?;
    crate::workflow::postprocess_chapter_after_content_change(state, accepted.chapter_id).await?;
    projections::import_review_summary(&accepted.review)
}

pub async fn batch_accept_import_reviews(
    state: &AppState,
    user_id: i64,
    input: BatchAcceptImportReviewsInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    let mut summaries = Vec::with_capacity(input.review_ids.len());
    for review_id in input.review_ids {
        summaries.push(accept_import_review(state, user_id, review_id).await?);
    }
    Ok(summaries)
}
