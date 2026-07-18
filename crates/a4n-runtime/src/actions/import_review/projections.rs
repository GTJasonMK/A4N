use super::{ImportAnalysisStatusSummary, ImportReviewSummary};
use a4n_core::{error::AppError, models::project::import_review_queue};
use a4n_orchestrator::import_review::normalize_import_review_status;
use std::collections::HashMap;

pub fn import_review_summary(
    review: &import_review_queue::Model,
) -> Result<ImportReviewSummary, AppError> {
    let review_status = normalized_review_status(review)?.to_string();
    Ok(ImportReviewSummary {
        id: review.id,
        project_id: review.project_id,
        filename: review.filename.clone(),
        file_type: review.file_type.clone(),
        detected_encoding: review.detected_encoding.clone(),
        parsed_chapters: review.parsed_chapters,
        dedup_paragraphs: review.dedup_paragraphs,
        dedup_rate: review.dedup_rate,
        analysis_result: review.analysis_result.clone(),
        review_status,
        created_at: Some(review.created_at.to_string()),
        updated_at: Some(review.updated_at.to_string()),
    })
}

pub(super) fn import_analysis_status_summaries(
    reviews: Vec<import_review_queue::Model>,
) -> Result<Vec<ImportAnalysisStatusSummary>, AppError> {
    let mut status_counts = HashMap::<String, i64>::new();
    for review in reviews {
        let status = normalized_review_status(&review)?.to_string();
        *status_counts.entry(status).or_insert(0) += 1;
    }
    let mut summaries = status_counts
        .into_iter()
        .map(|(status, count)| ImportAnalysisStatusSummary { status, count })
        .collect::<Vec<_>>();
    summaries.sort_by(|a, b| a.status.cmp(&b.status));
    Ok(summaries)
}

fn normalized_review_status(review: &import_review_queue::Model) -> Result<&'static str, AppError> {
    normalize_import_review_status(&review.review_status).map_err(|_| AppError::InvalidState {
        current: format!(
            "导入审核 #{} review_status={:?}",
            review.id, review.review_status
        ),
        operation: "读取导入审核状态".to_string(),
    })
}
