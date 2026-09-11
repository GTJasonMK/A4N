use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::{Map, Value};

use super::super::super::values::analysis_materialize_error;

mod item;
use item::materialize_metadata_entity;

pub(super) fn materialize_metadata_entities(
    metadata: &mut Map<String, Value>,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    for key in ["characters", "locations", "items"] {
        let values = metadata
            .get(key)
            .and_then(Value::as_array)
            .ok_or_else(|| analysis_materialize_error(format!("metadata.{key} 必须是数组")))?;
        let mut materialized = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            materialized.push(materialize_metadata_entity(
                value,
                key,
                index,
                source_spans,
            )?);
        }
        metadata.insert(key.to_string(), Value::Array(materialized));
    }
    Ok(())
}
