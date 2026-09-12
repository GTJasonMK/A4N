use a4n_db_models::models::chapter::{chapter_outline, part_outline};
use a4n_error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;

use super::ProgressReporter;

pub fn part_chapter_count(start_chapter: i32, end_chapter: i32) -> Result<i32, AppError> {
    if start_chapter <= 0 || end_chapter < start_chapter {
        return Err(AppError::InvalidArgument(format!(
            "分部章节范围无效：start_chapter={start_chapter}, end_chapter={end_chapter}"
        )));
    }
    let count = i64::from(end_chapter) - i64::from(start_chapter) + 1;
    i32::try_from(count).map_err(|_| {
        AppError::InvalidArgument(format!(
            "分部章节数量超出 i32 范围：start_chapter={start_chapter}, end_chapter={end_chapter}"
        ))
    })
}

pub fn progress_total(chapter_count: i32) -> Result<u32, AppError> {
    if chapter_count <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "章节大纲进度总量必须大于 0，实际为 {chapter_count}"
        )));
    }
    u32::try_from(chapter_count).map_err(|_| {
        AppError::InvalidArgument(format!("章节大纲进度总量超出 u32 范围：{chapter_count}"))
    })
}

pub fn batch_parts_progress_total(parts: &[part_outline::Model]) -> Result<u32, AppError> {
    if parts.is_empty() {
        return Err(AppError::InvalidArgument(
            "批量章节大纲生成至少需要一个分部".to_string(),
        ));
    }
    parts.iter().try_fold(0_u32, |total, part| {
        let count = part_chapter_count(part.start_chapter, part.end_chapter)?;
        total.checked_add(progress_total(count)?).ok_or_else(|| {
            AppError::InvalidArgument("批量章节大纲进度总量超出 u32 范围".to_string())
        })
    })
}

pub fn order_parts_for_batch_generation(parts: &mut [part_outline::Model]) {
    parts.sort_by_key(|part| (part.part_number, part.id));
}

pub fn report_progress(
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

pub fn ensure_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "模型生成{label}时达到 max_output_tokens 上限，返回 JSON 被截断；请提高模型输出上限，或减少生成范围/输出细节后重试"
        )));
    }
    Ok(())
}

pub fn validate_existing_chapter_outline_for_request(
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
