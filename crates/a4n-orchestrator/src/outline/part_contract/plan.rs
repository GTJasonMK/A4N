use super::model::PartGenerationPlan;
use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use std::collections::BTreeSet;

pub(in crate::outline) fn validate_part_generation_inputs(
    part_count: i32,
    chapters_per_part: i32,
) -> Result<(), AppError> {
    if part_count <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 part_count 必须大于 0".to_string(),
        ));
    }
    if chapters_per_part <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 chapters_per_part 必须大于 0".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::outline) fn resolve_part_generation_plan(
    requested_part_count: Option<i32>,
    requested_chapters_per_part: Option<i32>,
    project_chapters_per_part: Option<i32>,
    blueprint_chapters_per_part: Option<i32>,
) -> Result<PartGenerationPlan, AppError> {
    let requested_part_count = requested_part_count.unwrap_or(1);
    let chapters_per_part = requested_chapters_per_part
        .or(project_chapters_per_part)
        .or(blueprint_chapters_per_part)
        .ok_or_else(|| {
            AppError::InvalidArgument("项目缺少 chapters_per_part，无法生成分部大纲".to_string())
        })?;
    validate_part_generation_inputs(requested_part_count, chapters_per_part)?;

    Ok(PartGenerationPlan {
        chapters_per_part,
        requested_part_count,
    })
}

pub(in crate::outline) fn validate_existing_parts_for_plan(
    parts: &[part_outline::Model],
) -> Result<(), AppError> {
    validate_existing_parts(parts, "请先清理后再生成")
}

fn validate_existing_parts(parts: &[part_outline::Model], action: &str) -> Result<(), AppError> {
    let mut seen = BTreeSet::new();
    let mut ranges = Vec::with_capacity(parts.len());
    for part in parts {
        if !seen.insert(part.part_number) {
            return Err(AppError::InvalidArgument(format!(
                "第 {} 部已有重复分部大纲，{}",
                part.part_number, action
            )));
        }
        if part.part_number <= 0 {
            return Err(AppError::InvalidArgument(format!(
                "已有第 {} 部分部编号无效，{}",
                part.part_number, action
            )));
        }
        if part.start_chapter <= 0 || part.end_chapter < part.start_chapter {
            return Err(AppError::InvalidArgument(format!(
                "已有第 {} 部章节范围 {}..={} 无效，{}",
                part.part_number, part.start_chapter, part.end_chapter, action
            )));
        }
        ranges.push((part.start_chapter, part.end_chapter, part.part_number));
    }
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number)| {
        (*start_chapter, *end_chapter, *part_number)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part) = pair[0];
        let (start, end, part) = pair[1];
        if start <= prev_end {
            return Err(AppError::InvalidArgument(format!(
                "已有分部章节范围重叠：第 {prev_part} 部 {prev_start}..={prev_end} 与第 {part} 部 {start}..={end}，{action}"
            )));
        }
    }
    Ok(())
}
