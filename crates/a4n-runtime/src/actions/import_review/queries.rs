use super::{ImportAnalysisStatusSummary, ImportReviewSummary, access, projections};
use crate::{AppState, actions::access as project_access};
use a4n_core::error::AppError;

pub async fn list_import_queue(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let reviews = state
        .orchestrator
        .import_reviews_by_project(project_id)
        .await?;
    reviews
        .iter()
        .map(projections::import_review_summary)
        .collect()
}

pub async fn get_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    let review = access::verify_review_access(state, user_id, review_id).await?;
    projections::import_review_summary(&review)
}

pub async fn get_import_analysis_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ImportAnalysisStatusSummary>, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let reviews = state
        .orchestrator
        .import_reviews_by_project(project_id)
        .await?;
    projections::import_analysis_status_summaries(reviews)
}
