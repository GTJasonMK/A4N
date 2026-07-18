use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use a4n_llm::provider::is_truncated_finish_reason;

use super::ProgressReporter;

pub(in crate::outline) fn report_progress(
    progress: ProgressReporter<'_>,
    current: i32,
    total: i32,
    message: &str,
) -> Result<(), AppError> {
    if let Some(report) = progress {
        if total <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "大纲进度 total 必须大于 0，实际为 {total}"
            )));
        }
        if current < 0 || current > total {
            return Err(AppError::InvalidArgument(format!(
                "大纲进度 current 必须在 0..={total} 范围内，实际为 {current}"
            )));
        }
        let current = u32::try_from(current)
            .map_err(|_| AppError::InvalidArgument("大纲进度 current 超出 u32 范围".to_string()))?;
        let total = u32::try_from(total)
            .map_err(|_| AppError::InvalidArgument("大纲进度 total 超出 u32 范围".to_string()))?;
        report(current, total, message)?;
    }
    Ok(())
}

pub(in crate::outline) fn ensure_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "模型生成{label}时达到当前场景输出上限，返回 JSON 被截断；请在设置页调大大纲生成 max_tokens，或减少生成范围/输出细节后重试"
        )));
    }
    Ok(())
}

pub(in crate::outline) fn validate_existing_chapter_outline_for_request(
    outline: &chapter_outline::Model,
    chapter_number: i32,
    expected_part_id: Option<i64>,
) -> Result<(), AppError> {
    if outline.chapter_number != chapter_number {
        return Err(AppError::InvalidArgument(format!(
            "已有章纲章节号 {} 与当前请求第 {chapter_number} 章不一致",
            outline.chapter_number
        )));
    }
    if outline.part_id != expected_part_id {
        return Err(AppError::InvalidArgument(format!(
            "已有第 {chapter_number} 章章纲关联分部 {:?} 与当前计划 {:?} 不一致，请先删除旧章纲或重新生成对应范围",
            outline.part_id, expected_part_id
        )));
    }
    Ok(())
}
