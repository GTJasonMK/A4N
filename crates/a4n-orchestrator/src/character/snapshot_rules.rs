//! 角色快照查询输入门禁（无 DB）。

use a4n_error::AppError;
use serde_json::json;

pub const CHARACTER_SNAPSHOT_ENTITY_TYPE: &str = "character_profile";

pub fn initial_snapshot_data() -> serde_json::Value {
    json!({
        "synced_chapter": 0,
        "state_summary": "初始空状态",
        "changes_in_chapter": 0,
        "behaviors_in_chapter": 0,
        "attributes": {
            "explicit_attributes": {},
            "implicit_attributes": {},
            "social_attributes": {},
        },
        "explicit_attributes": {},
        "implicit_attributes": {},
        "social_attributes": {},
    })
}

/// 快照列表的可选章节范围。
pub fn validate_snapshot_range(
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<(), AppError> {
    if start_chapter.is_some_and(|value| value <= 0) {
        return Err(AppError::InvalidArgument(
            "快照起始章节必须大于 0".to_string(),
        ));
    }
    if end_chapter.is_some_and(|value| value <= 0) {
        return Err(AppError::InvalidArgument(
            "快照结束章节必须大于 0".to_string(),
        ));
    }
    if let (Some(start), Some(end)) = (start_chapter, end_chapter)
        && start > end
    {
        return Err(AppError::InvalidArgument(
            "快照起始章节不能大于结束章节".to_string(),
        ));
    }
    Ok(())
}

/// 单章快照查询：章节号必须为正。
pub fn ensure_positive_snapshot_chapter(chapter_number: i32) -> Result<(), AppError> {
    if chapter_number <= 0 {
        return Err(AppError::InvalidArgument("章节号必须大于 0".to_string()));
    }
    Ok(())
}

/// 章节 diff：from 可 0，to 必须为正且 from ≤ to。
pub fn validate_snapshot_diff_chapters(from_chapter: i32, to_chapter: i32) -> Result<(), AppError> {
    if from_chapter < 0 {
        return Err(AppError::InvalidArgument(
            "起始章节号不能小于 0".to_string(),
        ));
    }
    if to_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "目标章节号必须大于 0".to_string(),
        ));
    }
    if from_chapter > to_chapter {
        return Err(AppError::InvalidArgument(
            "起始章节号不能大于目标章节号".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_range_rejects_inverted_bounds() {
        let err = validate_snapshot_range(Some(5), Some(2)).expect_err("inverted");
        assert!(matches!(
            err,
            AppError::InvalidArgument(message) if message.contains("不能大于结束")
        ));
    }

    #[test]
    fn diff_chapters_allows_initial_from_zero() {
        assert!(validate_snapshot_diff_chapters(0, 3).is_ok());
        assert!(validate_snapshot_diff_chapters(-1, 3).is_err());
        assert!(validate_snapshot_diff_chapters(2, 1).is_err());
    }
}
