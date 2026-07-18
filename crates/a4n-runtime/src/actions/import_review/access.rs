use crate::{AppState, actions::access as project_access};
use a4n_core::{error::AppError, models::project::import_review_queue};

pub(super) async fn verify_review_access(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<import_review_queue::Model, AppError> {
    let review = state
        .orchestrator
        .import_review_by_id(review_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("导入审核不存在: review_id={review_id}")))?;
    project_access::verify_project_editor(state, user_id, review.project_id).await?;
    Ok(review)
}
