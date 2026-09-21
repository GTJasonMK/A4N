use a4n_error::AppError;
use serde_json::Value;

use super::super::required_stage_field;
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
            required_stage_field(facts, "chapter_fact_extraction", key)?,
        );
    }
    let tags = metadata_additions.get("tags").cloned().ok_or_else(|| {
        analysis_materialize_error("chapter_summary_synthesis.metadata_additions 缺少必需字段 tags")
    })?;
    metadata.insert("tags".to_string(), tags);
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
