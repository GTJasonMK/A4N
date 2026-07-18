use a4n_core::{error::AppError, source_spans::SourceSpan};
use serde_json::Value;

#[path = "rules.rs"]
mod rules;
use rules::validate_key_event_enums;

use super::super::super::values::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_source_span_ids, validate_string_array,
};

pub(super) fn materialize_key_event(
    event: &mut Value,
    index: usize,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let object = event.as_object_mut().ok_or_else(|| {
        analysis_materialize_error(format!("key_events[{index}] 必须是 JSON 对象"))
    })?;
    reject_materialize_unknown_fields(
        object,
        &[
            "type",
            "description",
            "importance",
            "involved_characters",
            "source_span_ids",
        ],
        &format!("key_events[{index}]"),
    )?;
    validate_key_event_enums(object, index)?;
    required_materialize_string(
        object,
        "description",
        &format!("key_events[{index}].description"),
    )?;
    validate_string_array(
        object.get("involved_characters"),
        &format!("key_events[{index}].involved_characters"),
    )?;
    validate_source_span_ids(
        object.get("source_span_ids"),
        source_spans,
        &format!("key_events[{index}].source_span_ids"),
    )?;
    object.remove("source_span_ids");
    Ok(())
}
