use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_required_string_enum, validate_string_array,
};

pub(super) fn validate_foreshadowing_candidate(
    item: &Value,
    index: usize,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    let path = format!("chapter_fact_extraction.foreshadowing_candidates[{index}]");
    let object = item
        .as_object()
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是对象")))?;
    reject_materialize_unknown_fields(
        object,
        &[
            "foreshadowing_ref",
            "description",
            "category",
            "priority",
            "related_entities",
            "original_text_span_id",
        ],
        &path,
    )?;
    required_materialize_string(
        object,
        "foreshadowing_ref",
        &format!("{path}.foreshadowing_ref"),
    )?;
    required_materialize_string(object, "description", &format!("{path}.description"))?;
    validate_required_string_enum(
        object.get("category"),
        &[
            "character_secret",
            "plot_twist",
            "item_mystery",
            "world_rule",
        ],
        &format!("{path}.category"),
    )?;
    validate_required_string_enum(
        object.get("priority"),
        &["high", "medium", "low"],
        &format!("{path}.priority"),
    )?;
    validate_string_array(
        object.get("related_entities"),
        &format!("{path}.related_entities"),
    )?;
    let span_id = required_materialize_string(
        object,
        "original_text_span_id",
        &format!("{path}.original_text_span_id"),
    )?;
    source_spans
        .resolve_text(&span_id)
        .map_err(analysis_materialize_error)
        .map(|_| ())
}
