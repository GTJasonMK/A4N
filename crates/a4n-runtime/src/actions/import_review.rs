use crate::AppState;
use crate::actions::access;
use a4n_core::error::AppError;
use a4n_core::models::{chapter::chapter, project::import_review_queue};
use a4n_services::import::{ModifyImportReviewInput, dedup, normalize_import_review_status};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReviewSummary {
    pub id: i64,
    pub project_id: i64,
    pub filename: String,
    pub file_type: String,
    pub detected_encoding: Option<String>,
    pub parsed_chapters: i32,
    pub dedup_paragraphs: i32,
    pub dedup_rate: Option<f32>,
    pub analysis_result: Option<serde_json::Value>,
    pub review_status: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchAcceptImportReviewsInput {
    pub review_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAnalysisStatusSummary {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DedupResult {
    pub content_hash: String,
    pub is_duplicate: bool,
    pub closest_distance: u32,
    pub matched_chapter_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateImportStatusInput {
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CheckImportDedupInput {
    pub content: String,
    pub project_id: i64,
}

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

pub async fn list_import_queue(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ImportReviewSummary>, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let reviews = state.import.list_queue(project_id).await?;
    reviews.iter().map(import_review_summary).collect()
}

pub async fn get_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    let review = verify_review_access(state, user_id, review_id).await?;
    import_review_summary(&review)
}

pub async fn update_import_status(
    state: &AppState,
    user_id: i64,
    review_id: i64,
    input: UpdateImportStatusInput,
) -> Result<ImportReviewSummary, AppError> {
    verify_review_access(state, user_id, review_id).await?;
    let updated = state.import.update_status(review_id, &input.status).await?;
    import_review_summary(&updated)
}

pub async fn delete_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<(), AppError> {
    verify_review_access(state, user_id, review_id).await?;
    state.import.delete_review(review_id).await
}

pub async fn accept_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    let review = verify_review_access(state, user_id, review_id).await?;
    let accepted = state.import.accept_review(review).await?;
    crate::workflow::postprocess_chapter_after_content_change(state, accepted.chapter_id).await?;
    import_review_summary(&accepted.review)
}

pub async fn modify_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
    input: ModifyImportReviewInput,
) -> Result<ImportReviewSummary, AppError> {
    let review = verify_review_access(state, user_id, review_id).await?;
    let accepted = state.import.modify_review(review, input).await?;
    crate::workflow::postprocess_chapter_after_content_change(state, accepted.chapter_id).await?;
    import_review_summary(&accepted.review)
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

pub async fn reject_import_review(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<ImportReviewSummary, AppError> {
    verify_review_access(state, user_id, review_id).await?;
    let updated = state.import.update_status(review_id, "rejected").await?;
    import_review_summary(&updated)
}

pub async fn get_import_analysis_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ImportAnalysisStatusSummary>, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    let reviews = import_review_queue::Entity::find()
        .filter(import_review_queue::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    import_analysis_status_summaries(reviews)
}

pub async fn check_import_dedup(
    state: &AppState,
    user_id: i64,
    input: CheckImportDedupInput,
) -> Result<DedupResult, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let content_hash = dedup::compute_simhash(&input.content, 3);
    let existing_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(input.project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .all(state.db.as_ref())
        .await
        .map_err(AppError::Database)?;

    let threshold = 3;
    let mut closest_distance = u32::MAX;
    let mut matched_chapter_id = None;
    for chapter in existing_chapters {
        let existing_hash = dedup::compute_simhash(&chapter.content, 3);
        let distance = content_hash.hamming_distance(&existing_hash);
        if distance < closest_distance {
            closest_distance = distance;
        }
        if distance <= threshold && matched_chapter_id.is_none() {
            matched_chapter_id = Some(chapter.id);
        }
    }

    Ok(DedupResult {
        content_hash: format!("{:016x}", content_hash.0),
        is_duplicate: matched_chapter_id.is_some(),
        closest_distance,
        matched_chapter_id,
    })
}

async fn verify_review_access(
    state: &AppState,
    user_id: i64,
    review_id: i64,
) -> Result<import_review_queue::Model, AppError> {
    let review = import_review_queue::Entity::find_by_id(review_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("导入审核不存在: review_id={review_id}")))?;
    access::verify_project_editor(state, user_id, review.project_id).await?;
    Ok(review)
}

fn import_analysis_status_summaries(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_review_summary_rejects_unknown_review_status() {
        let review = review_with_status(7, "approved");

        let err = import_review_summary(&review).expect_err("legacy status should fail");

        assert!(matches!(err, AppError::InvalidState { .. }));
        assert!(err.to_string().contains("导入审核 #7"));
        assert!(err.to_string().contains("approved"));
    }

    #[test]
    fn import_analysis_status_summaries_rejects_unknown_review_status() {
        let err = import_analysis_status_summaries(vec![review_with_status(8, "archived")])
            .expect_err("unknown status should fail");

        assert!(matches!(err, AppError::InvalidState { .. }));
        assert!(err.to_string().contains("导入审核 #8"));
        assert!(err.to_string().contains("archived"));
    }

    #[test]
    fn import_analysis_status_summaries_normalizes_trimmed_statuses() {
        let summaries = import_analysis_status_summaries(vec![
            review_with_status(1, "pending"),
            review_with_status(2, " pending\n"),
            review_with_status(3, "rejected"),
        ])
        .expect("valid statuses should summarize");

        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].status, "pending");
        assert_eq!(summaries[0].count, 2);
        assert_eq!(summaries[1].status, "rejected");
        assert_eq!(summaries[1].count, 1);
    }

    fn review_with_status(id: i64, status: &str) -> import_review_queue::Model {
        let timestamp = chrono::NaiveDate::from_ymd_opt(2026, 6, 29)
            .expect("valid date")
            .and_hms_opt(23, 30, 0)
            .expect("valid time");
        import_review_queue::Model {
            id,
            project_id: 42,
            filename: "demo.md".to_string(),
            file_type: "markdown".to_string(),
            detected_encoding: Some("utf-8".to_string()),
            parsed_chapters: 1,
            dedup_paragraphs: 1,
            dedup_rate: Some(0.0),
            analysis_result: Some(serde_json::json!({"chapter_number": 1})),
            review_status: status.to_string(),
            created_at: timestamp,
            updated_at: timestamp,
        }
    }
}
