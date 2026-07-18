use a4n_core::blueprint::validate_blueprint_chapter_count;
use a4n_core::error::AppError;
use a4n_db::project::needs_part_outlines_for_chapters;
use serde_json::Value;

use super::super::super::{
    CHAPTERS_PER_PART,
    fields::{required_array, required_bool, required_i32},
};

pub(super) fn parse_blueprint_derived_fields(
    raw: &Value,
    requested_chapters: i32,
) -> Result<(i32, bool, i32), AppError> {
    let total_chapters = required_i32(raw, "total_chapters", "blueprint")?;
    validate_blueprint_chapter_count(total_chapters, "blueprint.total_chapters")?;
    if total_chapters != requested_chapters {
        return Err(AppError::InvalidArgument(format!(
            "blueprint.total_chapters 必须等于 requested_chapter_count {requested_chapters}，实际为 {total_chapters}"
        )));
    }
    let needs_part_outlines = required_bool(raw, "needs_part_outlines", "blueprint")?;
    let expected_needs_part_outlines = needs_part_outlines_for_chapters(total_chapters);
    if needs_part_outlines != expected_needs_part_outlines {
        return Err(AppError::InvalidArgument(format!(
            "blueprint.needs_part_outlines 必须为 {expected_needs_part_outlines}"
        )));
    }
    let chapters_per_part = required_i32(raw, "chapters_per_part", "blueprint")?;
    if chapters_per_part != CHAPTERS_PER_PART {
        return Err(AppError::InvalidArgument(format!(
            "blueprint.chapters_per_part 必须为 {CHAPTERS_PER_PART}"
        )));
    }
    if !required_array(raw, "chapter_outline", "blueprint")?.is_empty() {
        return Err(AppError::InvalidArgument(
            "blueprint.chapter_outline 必须为空数组 []".to_string(),
        ));
    }
    Ok((total_chapters, needs_part_outlines, chapters_per_part))
}
