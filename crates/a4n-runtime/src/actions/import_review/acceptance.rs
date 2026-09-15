use super::{AcceptImportReviewInput, BatchAcceptImportReviewsInput, ImportReviewSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn accept_import_review(
    state: &AppState,
    user_id: i64,
    input: AcceptImportReviewInput,
) -> Result<ImportReviewSummary, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .import_review_acceptance_application()
        .accept(user_id, input, &host_port)
        .await
}

pub async fn batch_accept_import_reviews(
    state: &AppState,
    user_id: i64,
    input: BatchAcceptImportReviewsInput,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    let host_port = crate::workflow::RuntimeChapterAnalysisHostPort::new(state);
    state
        .import_review_acceptance_application()
        .batch_accept(user_id, input, &host_port)
        .await
}
