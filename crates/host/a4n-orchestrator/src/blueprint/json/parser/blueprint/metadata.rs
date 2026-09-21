use a4n_error::AppError;
use serde_json::Value;

use super::super::super::fields::required_string;

pub(super) fn parse_blueprint_metadata(
    raw: &Value,
) -> Result<(String, String, String, String, String, String, String), AppError> {
    Ok((
        required_string(raw, "title", "blueprint")?,
        required_string(raw, "genre", "blueprint")?,
        required_string(raw, "style", "blueprint")?,
        required_string(raw, "tone", "blueprint")?,
        required_string(raw, "target_audience", "blueprint")?,
        required_string(raw, "one_sentence_summary", "blueprint")?,
        required_string(raw, "full_synopsis", "blueprint")?,
    ))
}
