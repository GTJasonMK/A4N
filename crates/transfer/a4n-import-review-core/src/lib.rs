//! Pure import-review projections and duplicate detection.

use a4n_api_contract::import_review::{DedupResult, ImportReviewSummary};
use a4n_db_models::models::project::import_review_queue;
use a4n_error::AppError;
use a4n_import_core::dedup;

const SIMHASH_NGRAM: usize = 3;
const DUPLICATE_HAMMING_THRESHOLD: u32 = 3;

pub fn summary_from_model(
    review: &import_review_queue::Model,
) -> Result<ImportReviewSummary, AppError> {
    let review_status = a4n_domain::import_review::ImportReviewStatus::parse(&review.review_status)
        .ok_or_else(|| AppError::InvalidState {
            current: format!(
                "导入审核 #{} review_status={:?}",
                review.id, review.review_status
            ),
            operation: "读取导入审核状态".to_string(),
        })?;
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

pub fn calculate_dedup(
    content: &str,
    existing_chapters: &[(i64, String)],
) -> Result<DedupResult, AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "待检测导入正文不能为空".to_string(),
        ));
    }
    let content_hash = dedup::compute_simhash(content, SIMHASH_NGRAM);
    let closest = existing_chapters
        .iter()
        .map(|(chapter_id, chapter_content)| {
            let existing_hash = dedup::compute_simhash(chapter_content, SIMHASH_NGRAM);
            (*chapter_id, content_hash.hamming_distance(&existing_hash))
        })
        .min_by_key(|(_, distance)| *distance);
    let closest_distance = closest.map(|(_, distance)| distance);
    let matched_chapter_id = closest
        .filter(|(_, distance)| *distance <= DUPLICATE_HAMMING_THRESHOLD)
        .map(|(chapter_id, _)| chapter_id);
    Ok(DedupResult {
        content_hash: format!("{:016x}", content_hash.0),
        is_duplicate: matched_chapter_id.is_some(),
        closest_distance,
        matched_chapter_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_content_returns_match_and_distance() {
        let result = calculate_dedup(
            "这是一段用于导入去重的正文。",
            &[
                (7, "完全不同的章节。".to_string()),
                (9, "这是一段用于导入去重的正文。".to_string()),
            ],
        )
        .expect("dedup input should be valid");

        assert!(result.is_duplicate);
        assert_eq!(result.matched_chapter_id, Some(9));
        assert_eq!(result.closest_distance, Some(0));
        assert_eq!(result.content_hash.len(), 16);
    }

    #[test]
    fn blank_content_is_rejected() {
        let error = calculate_dedup(" \n", &[]).expect_err("blank content must fail");
        assert!(
            matches!(error, AppError::InvalidArgument(message) if message.contains("不能为空"))
        );
    }
}
