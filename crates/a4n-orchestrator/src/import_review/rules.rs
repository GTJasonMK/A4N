//! 导入审核 application 输入门禁（无 DB）。

use a4n_core::{error::AppError, models::project::import_review_queue};
use a4n_db::import_review::ModifyImportReviewInput;
use a4n_import_core::dedup;

use super::ImportDedupResult;

const SIMHASH_NGRAM: usize = 3;
const DUPLICATE_HAMMING_THRESHOLD: u32 = 3;

pub(crate) fn calculate_import_dedup(
    content: &str,
    existing_chapters: &[(i64, String)],
) -> Result<ImportDedupResult, AppError> {
    if content.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "待检测导入正文不能为空".to_string(),
        ));
    }
    let content_hash = dedup::compute_simhash(content, SIMHASH_NGRAM);
    let mut closest_distance = u32::MAX;
    let mut matched_chapter_id = None;
    for (chapter_id, chapter_content) in existing_chapters {
        let existing_hash = dedup::compute_simhash(chapter_content, SIMHASH_NGRAM);
        let distance = content_hash.hamming_distance(&existing_hash);
        closest_distance = closest_distance.min(distance);
        if distance <= DUPLICATE_HAMMING_THRESHOLD && matched_chapter_id.is_none() {
            matched_chapter_id = Some(*chapter_id);
        }
    }
    Ok(ImportDedupResult {
        content_hash: format!("{:016x}", content_hash.0),
        is_duplicate: matched_chapter_id.is_some(),
        closest_distance,
        matched_chapter_id,
    })
}

/// 接受前必须已有分析结果。
pub(crate) fn ensure_import_review_ready_for_accept(
    review: &import_review_queue::Model,
) -> Result<(), AppError> {
    if review.analysis_result.is_none() {
        return Err(AppError::InvalidArgument(format!(
            "导入审核缺少分析结果: review_id={}",
            review.id
        )));
    }
    Ok(())
}

/// 规范化修改并接受的输入：trim 正文与摘要空白。
pub(crate) fn normalize_modify_import_review_input(
    input: ModifyImportReviewInput,
) -> ModifyImportReviewInput {
    ModifyImportReviewInput {
        content: input
            .content
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        analysis_result: input.analysis_result,
        ai_summary: input
            .ai_summary
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        ai_characters: input.ai_characters,
        ai_world_setting: input.ai_world_setting,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ensure_ready_for_accept_rejects_missing_analysis() {
        let review = import_review_queue::Model {
            id: 9,
            project_id: 1,
            filename: "a.txt".into(),
            file_type: "txt".into(),
            detected_encoding: None,
            parsed_chapters: 1,
            dedup_paragraphs: 0,
            dedup_rate: None,
            analysis_result: None,
            review_status: "reviewing".into(),
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };
        let err = ensure_import_review_ready_for_accept(&review).expect_err("missing analysis");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message.contains("缺少分析结果")
        ));
    }

    #[test]
    fn normalize_modify_trims_and_drops_blank_text() {
        let input = normalize_modify_import_review_input(ModifyImportReviewInput {
            content: Some("  正文  ".into()),
            analysis_result: Some(json!({})),
            ai_summary: Some("   ".into()),
            ai_characters: None,
            ai_world_setting: None,
        });
        assert_eq!(input.content.as_deref(), Some("正文"));
        assert_eq!(input.ai_summary, None);
        assert!(input.analysis_result.is_some());
    }

    #[test]
    fn calculate_import_dedup_returns_match_and_nearest_distance() {
        let result = calculate_import_dedup(
            "这是一段用于导入去重的正文。",
            &[
                (7, "完全不同的章节。".to_string()),
                (9, "这是一段用于导入去重的正文。".to_string()),
            ],
        )
        .expect("dedup input should be valid");

        assert!(result.is_duplicate);
        assert_eq!(result.matched_chapter_id, Some(9));
        assert_eq!(result.closest_distance, 0);
        assert_eq!(result.content_hash.len(), 16);
    }

    #[test]
    fn calculate_import_dedup_rejects_blank_content() {
        let error = calculate_import_dedup(" \n", &[]).expect_err("blank content must fail");

        assert!(
            matches!(error, AppError::InvalidArgument(message) if message.contains("不能为空"))
        );
    }
}
