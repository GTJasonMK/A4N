use a4n_core::{error::AppError, source_spans::SourceSpan};
use serde_json::Value;

use super::super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_source_span_ids,
};

pub(super) fn validate_fact_entity(
    value: &Value,
    key: &str,
    index: usize,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let path = format!("chapter_fact_extraction.{key}[{index}]");
    let object = value
        .as_object()
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是对象")))?;
    reject_materialize_unknown_fields(object, &["name", "source_span_ids"], &path)?;
    required_materialize_string(object, "name", &format!("{path}.name"))?;
    validate_source_span_ids(
        object.get("source_span_ids"),
        source_spans,
        &format!("{path}.source_span_ids"),
    )
}
