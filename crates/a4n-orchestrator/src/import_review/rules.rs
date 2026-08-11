//! 导入审核 application 输入门禁（无 DB）。

use a4n_core::error::AppError;
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

#[cfg(test)]
mod tests {
    use super::*;

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
