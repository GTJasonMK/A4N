use a4n_error::AppError;
use serde_json::Value;

use super::common::{
    reject_manga_unknown_fields, required_manga_string, validate_manga_positive_i64,
    validate_manga_string_array,
};

pub(in crate::manga) fn validate_manga_chapter_comic_synthesis(
    synthesis_json: &Value,
) -> Result<(), AppError> {
    reject_manga_unknown_fields(
        synthesis_json,
        &[
            "chapter_summary",
            "mood_progression",
            "total_estimated_pages",
        ],
        "manga_chapter_comic_synthesis",
    )?;
    required_manga_string(
        synthesis_json,
        "chapter_summary",
        "manga_chapter_comic_synthesis",
    )?;
    validate_manga_string_array(
        synthesis_json.get("mood_progression"),
        "manga_chapter_comic_synthesis.mood_progression",
    )?;
    let total = validate_manga_positive_i64(
        synthesis_json.get("total_estimated_pages"),
        "manga_chapter_comic_synthesis.total_estimated_pages",
    )?;
    if !(5..=15).contains(&total) {
        return Err(AppError::InvalidArgument(format!(
            "manga_chapter_comic_synthesis.total_estimated_pages 必须在 5-15 之间，实际为 {total}"
        )));
    }
    Ok(())
}
