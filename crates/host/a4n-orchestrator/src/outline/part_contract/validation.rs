use a4n_domain::chapter::PartOutlineRecord;
use a4n_error::AppError;

pub fn validate_part_outline_request(
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
) -> Result<(), AppError> {
    if part_number <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 part_number 必须大于 0".to_string(),
        ));
    }
    if start_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "分部大纲 start_chapter 必须大于 0".to_string(),
        ));
    }
    if end_chapter < start_chapter {
        return Err(AppError::InvalidArgument(
            "分部大纲 end_chapter 不能小于 start_chapter".to_string(),
        ));
    }
    Ok(())
}

pub fn ensure_continue_part_request(
    existing_parts: &[PartOutlineRecord],
    part_number: i32,
    start_chapter: i32,
) -> Result<(), AppError> {
    if let Some(last_part) = existing_parts.last() {
        let expected_part_number = last_part.part_number + 1;
        if part_number != expected_part_number {
            return Err(AppError::InvalidArgument(format!(
                "续作分部 part_number 必须等于 {expected_part_number}"
            )));
        }
        let expected_start = last_part.end_chapter + 1;
        if start_chapter != expected_start {
            return Err(AppError::InvalidArgument(format!(
                "续作分部 start_chapter 必须等于 {expected_start}"
            )));
        }
    } else {
        if part_number != 1 {
            return Err(AppError::InvalidArgument(
                "首个分部 part_number 必须等于 1".to_string(),
            ));
        }
        if start_chapter != 1 {
            return Err(AppError::InvalidArgument(
                "首个分部 start_chapter 必须等于 1".to_string(),
            ));
        }
    }
    Ok(())
}

pub fn ensure_no_part_outline_conflict(
    existing_parts: &[PartOutlineRecord],
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    exclude_part_id: Option<i64>,
) -> Result<(), AppError> {
    for part in existing_parts {
        if exclude_part_id == Some(part.id) {
            continue;
        }
        if part.part_number == part_number {
            return Err(AppError::InvalidArgument(format!(
                "分部编号已存在: {part_number}"
            )));
        }
        if ranges_overlap(
            start_chapter,
            end_chapter,
            part.start_chapter,
            part.end_chapter,
        ) {
            return Err(AppError::InvalidArgument(format!(
                "分部章节范围 {start_chapter}..={end_chapter} 与第 {} 分部 {}..={} 重叠",
                part.part_number, part.start_chapter, part.end_chapter
            )));
        }
    }
    Ok(())
}

fn ranges_overlap(left_start: i32, left_end: i32, right_start: i32, right_end: i32) -> bool {
    left_start <= right_end && right_start <= left_end
}
