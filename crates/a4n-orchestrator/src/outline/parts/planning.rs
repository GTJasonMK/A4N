use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;

pub(super) fn plan_next_parts(
    existing_parts: &[part_outline::Model],
    part_count: i32,
    chapters_per_part: i32,
) -> Result<Vec<(i32, i32, i32)>, AppError> {
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

    let (next_part_number, next_start_chapter) = existing_parts
        .iter()
        .max_by_key(|part| part.part_number)
        .map(|part| (part.part_number + 1, part.end_chapter + 1))
        .unwrap_or((1, 1));
    Ok((0..part_count)
        .map(|offset| {
            let part_number = next_part_number + offset;
            let start_chapter = next_start_chapter + offset * chapters_per_part;
            let end_chapter = start_chapter + chapters_per_part - 1;
            (part_number, start_chapter, end_chapter)
        })
        .collect())
}

pub(super) fn plan_replacement_parts(
    part_count: i32,
    chapters_per_part: i32,
) -> Result<Vec<(i32, i32, i32)>, AppError> {
    plan_next_parts(&[], part_count, chapters_per_part)
}
