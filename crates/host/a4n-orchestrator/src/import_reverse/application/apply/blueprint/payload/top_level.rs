use serde_json::Value;

use crate::import_reverse::application::reverse_json::reject_unknown_keys;

pub(in crate::import_reverse::application::apply) fn validate_reverse_blueprint_top_level_keys(
    data: &Value,
) -> Result<(), String> {
    reject_unknown_keys(
        data,
        &[
            "title",
            "genre",
            "style",
            "tone",
            "target_audience",
            "one_sentence_summary",
            "full_synopsis",
            "world_setting",
            "characters",
            "relationships",
            "total_chapters",
            "needs_part_outlines",
        ],
        "reverse_blueprint",
    )
}

pub(in crate::import_reverse::application::apply) fn optional_needs_part_outlines(
    data: &Value,
) -> Result<Option<bool>, String> {
    let Some(value) = data.get("needs_part_outlines") else {
        return Ok(None);
    };
    value
        .as_bool()
        .map(Some)
        .ok_or_else(|| "reverse_blueprint.needs_part_outlines 必须是布尔值".to_string())
}

#[cfg(test)]
mod tests;
