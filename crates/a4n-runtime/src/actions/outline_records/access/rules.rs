use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;

pub(in crate::actions::outline_records) fn part_chapter_count(
    start_chapter: i32,
    end_chapter: i32,
) -> Result<i32, AppError> {
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

pub(in crate::actions::outline_records) fn progress_total(
    chapter_count: i32,
) -> Result<u32, AppError> {
    if chapter_count <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "章节大纲进度总量必须大于 0，实际为 {chapter_count}"
        )));
    }
    u32::try_from(chapter_count).map_err(|_| {
        AppError::InvalidArgument(format!("章节大纲进度总量超出 u32 范围：{chapter_count}"))
    })
}

pub(in crate::actions::outline_records) fn batch_parts_progress_total(
    parts: &[part_outline::Model],
) -> Result<u32, AppError> {
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

pub(in crate::actions::outline_records) fn required_outline_pacing_score(
    pacing_score: Option<f32>,
    chapter_number: i32,
) -> Result<f32, AppError> {
    let score = pacing_score.ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "第 {chapter_number} 章缺少 pacing_score，无法分析节奏"
        ))
    })?;
    if score.is_finite() {
        Ok(score)
    } else {
        Err(AppError::InvalidArgument(format!(
            "第 {chapter_number} 章 pacing_score 不是有限数字，无法分析节奏"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::{part_chapter_count, progress_total};

    #[test]
    fn part_chapter_count_rejects_invalid_ranges() {
        assert!(part_chapter_count(0, 1).is_err());
        assert!(part_chapter_count(2, 1).is_err());
        assert_eq!(
            part_chapter_count(1, i32::MAX).expect("largest valid positive range"),
            i32::MAX
        );
    }

    #[test]
    fn progress_total_rejects_non_positive_counts() {
        assert!(progress_total(0).is_err());
        assert!(progress_total(-1).is_err());
        assert_eq!(progress_total(3).expect("valid progress total"), 3);
    }
}
