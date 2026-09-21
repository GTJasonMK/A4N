use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::values::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_source_span_ids,
};

pub(super) fn materialize_metadata_entity(
    value: &Value,
    key: &str,
    index: usize,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<Value, AppError> {
    let object = value.as_object().ok_or_else(|| {
        analysis_materialize_error(format!(
            "metadata.{key}[{index}] 必须是包含 name 和 source_span_ids 的对象"
        ))
    })?;
    reject_materialize_unknown_fields(
        object,
        &["name", "source_span_ids"],
        &format!("metadata.{key}[{index}]"),
    )?;
    let name =
        required_materialize_string(object, "name", &format!("metadata.{key}[{index}].name"))?;
    validate_source_span_ids(
        object.get("source_span_ids"),
        source_spans,
        &format!("metadata.{key}[{index}].source_span_ids"),
    )?;
    Ok(Value::String(name))
}
