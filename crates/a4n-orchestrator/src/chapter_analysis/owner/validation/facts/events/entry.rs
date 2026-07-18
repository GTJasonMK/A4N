use a4n_core::{error::AppError, source_spans::SourceSpan};
use serde_json::Value;

use super::super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_required_string_enum, validate_source_span_ids, validate_string_array,
};

pub(super) fn validate_event_candidate(
    event: &Value,
    index: usize,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let path = format!("chapter_fact_extraction.event_candidates[{index}]");
    let object = event
        .as_object()
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是对象")))?;
    reject_materialize_unknown_fields(
        object,
        &[
            "event_ref",
            "type",
            "description",
            "importance",
            "involved_characters",
            "source_span_ids",
        ],
        &path,
    )?;
    required_materialize_string(object, "event_ref", &format!("{path}.event_ref"))?;
    required_materialize_string(object, "description", &format!("{path}.description"))?;
    validate_required_string_enum(
        object.get("type"),
        &[
            "battle",
            "revelation",
            "relationship",
            "discovery",
            "decision",
            "death",
            "arrival",
            "departure",
        ],
        &format!("{path}.type"),
    )?;
    validate_required_string_enum(
        object.get("importance"),
        &["high", "medium", "low"],
        &format!("{path}.importance"),
    )?;
    validate_string_array(
        object.get("involved_characters"),
        &format!("{path}.involved_characters"),
    )?;
    validate_source_span_ids(
        object.get("source_span_ids"),
        source_spans,
        &format!("{path}.source_span_ids"),
    )
}
