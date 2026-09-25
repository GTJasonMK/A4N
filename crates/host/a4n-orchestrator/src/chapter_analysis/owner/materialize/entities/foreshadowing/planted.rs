use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::values::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_required_string_enum, validate_string_array,
};

pub(super) fn materialize_planted_entry(
    item: &mut Value,
    index: usize,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    let object = item.as_object_mut().ok_or_else(|| {
        analysis_materialize_error(format!("foreshadowing.planted[{index}] 必须是 JSON 对象"))
    })?;
    reject_materialize_unknown_fields(
        object,
        &[
            "id",
            "description",
            "original_text_span_id",
            "category",
            "priority",
            "related_entities",
        ],
        &format!("foreshadowing.planted[{index}]"),
    )?;
    required_materialize_string(
        object,
        "description",
        &format!("foreshadowing.planted[{index}].description"),
    )?;
    validate_required_string_enum(
        object.get("category"),
        &[
            "character_secret",
            "plot_twist",
            "item_mystery",
            "world_rule",
        ],
        &format!("foreshadowing.planted[{index}].category"),
    )?;
    validate_required_string_enum(
        object.get("priority"),
        &["high", "medium", "low"],
        &format!("foreshadowing.planted[{index}].priority"),
    )?;
    validate_string_array(
        object.get("related_entities"),
        &format!("foreshadowing.planted[{index}].related_entities"),
    )?;
    let span_id = required_materialize_string(
        object,
        "original_text_span_id",
        &format!("foreshadowing.planted[{index}].original_text_span_id"),
    )?;
    let original_text = source_spans
        .resolve_text(&span_id)
        .map_err(analysis_materialize_error)?;
    object.insert("original_text".to_string(), Value::String(original_text));
    object.remove("original_text_span_id");
    Ok(())
}
