use a4n_core::error::AppError;
use a4n_core::source_spans::SourceSpan;
use serde_json::Value;

use super::super::super::materialize::analysis_materialize_error;

mod entry;
use entry::validate_fact_entity;

pub(super) fn validate_fact_entity_array(
    facts: &Value,
    key: &str,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let values = facts.get(key).and_then(Value::as_array).ok_or_else(|| {
        analysis_materialize_error(format!("chapter_fact_extraction.{key} 必须是数组"))
    })?;
    for (index, value) in values.iter().enumerate() {
        validate_fact_entity(value, key, index, source_spans)?;
    }
    Ok(())
}
