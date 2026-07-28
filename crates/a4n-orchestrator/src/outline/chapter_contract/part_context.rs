use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;

pub fn validate_requested_part_context(
    parts: &[part_outline::Model],
    start_chapter: i32,
    end_chapter: i32,
) -> Result<(), AppError> {
    if parts.is_empty() {
        return Ok(());
    }

    for chapter_number in start_chapter..=end_chapter {
        if let Some(part) = requested_part_for_chapter(parts, chapter_number)? {
            required_part_summary(part)?;
        } else {
            return Err(AppError::InvalidArgument(format!(
                "第 {chapter_number} 章不在任何分部范围内，无法生成章节大纲"
            )));
        }
    }
    Ok(())
}

pub fn requested_part_for_chapter(
    parts: &[part_outline::Model],
    chapter_number: i32,
) -> Result<Option<&part_outline::Model>, AppError> {
    let mut matched = parts
        .iter()
        .filter(|part| chapter_number >= part.start_chapter && chapter_number <= part.end_chapter);
    let first = matched.next();
    if matched.next().is_some() {
        return Err(AppError::InvalidArgument(format!(
            "第 {chapter_number} 章匹配到多个分部大纲，无法生成章节大纲"
        )));
    }
    Ok(first)
}

pub fn ensure_focus_part_in_context(
    focus_part: Option<&part_outline::Model>,
    parts: &[part_outline::Model],
) -> Result<(), AppError> {
    let Some(focus_part) = focus_part else {
        return Ok(());
    };
    if parts.iter().any(|part| part.id == focus_part.id) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "分部大纲 {} 不属于当前章节大纲生成上下文",
            focus_part.id
        )))
    }
}

pub fn required_part_summary(part: &part_outline::Model) -> Result<&str, AppError> {
    part.summary
        .as_deref()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "分部大纲 {} 缺少 summary，无法生成章节大纲",
                part.part_number
            ))
        })
}

pub fn validate_chapter_generation_range(
    part: &part_outline::Model,
    start_chapter: i32,
    count: i32,
) -> Result<(i32, i32), AppError> {
    if count <= 0 {
        return Err(AppError::InvalidArgument(
            "章节大纲生成 count 必须大于 0".to_string(),
        ));
    }
    if start_chapter < part.start_chapter || start_chapter > part.end_chapter {
        return Err(AppError::InvalidArgument(format!(
            "章节大纲生成 start_chapter 必须在分部范围 {}..={} 内",
            part.start_chapter, part.end_chapter
        )));
    }
    let end_chapter = start_chapter
        .checked_add(count - 1)
        .ok_or_else(|| AppError::InvalidArgument("章节大纲生成范围溢出".to_string()))?;
    if end_chapter > part.end_chapter {
        return Err(AppError::InvalidArgument(format!(
            "章节大纲生成范围 {}..={} 超出分部范围 {}..={}",
            start_chapter, end_chapter, part.start_chapter, part.end_chapter
        )));
    }
    Ok((start_chapter, end_chapter))
}
