use super::ImportReviewSummary;
use a4n_core::{error::AppError, models::project::import_review_queue};
use a4n_domain::import_review::ImportReviewStatus;

pub(crate) fn import_review_summary(
    review: &import_review_queue::Model,
) -> Result<ImportReviewSummary, AppError> {
    let review_status = normalized_review_status(review)?;
    Ok(ImportReviewSummary {
        id: review.id,
        project_id: review.project_id,
        filename: review.filename.clone(),
        analysis_result: review.analysis_result.clone(),
        review_status,
        created_at: review.created_at.to_string(),
        updated_at: review.updated_at.to_string(),
    })
}

fn normalized_review_status(
    review: &import_review_queue::Model,
) -> Result<ImportReviewStatus, AppError> {
    ImportReviewStatus::parse(&review.review_status).ok_or_else(|| AppError::InvalidState {
        current: format!(
            "导入审核 #{} review_status={:?}",
            review.id, review.review_status
        ),
        operation: "读取导入审核状态".to_string(),
    })
}
