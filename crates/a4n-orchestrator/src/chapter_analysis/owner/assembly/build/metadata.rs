use a4n_core::error::AppError;
use serde_json::Value;

use crate::chapter_analysis::owner::materialize::analysis_materialize_error;

pub(super) fn assemble_metadata(facts: &Value, summary: &Value) -> Result<Value, AppError> {
    let metadata_additions = summary
        .get("metadata_additions")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_summary_synthesis.metadata_additions 必须是对象")
        })?;
    let mut metadata = serde_json::Map::new();
    for key in ["characters", "locations", "items"] {
        metadata.insert(
            key.to_string(),
            facts
                .get(key)
                .cloned()
                .unwrap_or_else(|| Value::Array(Vec::new())),
        );
    }
    metadata.insert(
        "tags".to_string(),
        metadata_additions
            .get("tags")
            .cloned()
            .unwrap_or_else(|| Value::Array(Vec::new())),
    );
    metadata.insert(
        "tone".to_string(),
        metadata_additions
            .get("tone")
            .cloned()
            .unwrap_or(Value::Null),
    );
    metadata.insert(
        "timeline_marker".to_string(),
        metadata_additions
            .get("timeline_marker")
            .cloned()
            .unwrap_or(Value::Null),
    );
    Ok(Value::Object(metadata))
}
