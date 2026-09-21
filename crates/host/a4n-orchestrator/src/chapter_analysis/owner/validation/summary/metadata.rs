use a4n_error::AppError;
use serde_json::Value;

use super::super::super::materialize::{
    analysis_materialize_error, reject_materialize_unknown_fields, validate_string_array,
};
use super::super::common::validate_optional_stage_string;

pub(super) fn validate_metadata_additions(value: Option<&Value>) -> Result<(), AppError> {
    let metadata = value.and_then(Value::as_object).ok_or_else(|| {
        analysis_materialize_error("chapter_summary_synthesis.metadata_additions 必须是对象")
    })?;
    reject_materialize_unknown_fields(
        metadata,
        &["tags", "tone", "timeline_marker"],
        "chapter_summary_synthesis.metadata_additions",
    )?;
    validate_string_array(
        metadata.get("tags"),
        "chapter_summary_synthesis.metadata_additions.tags",
    )?;
    validate_optional_stage_string(
        metadata.get("tone"),
        "chapter_summary_synthesis.metadata_additions.tone",
    )?;
    validate_optional_stage_string(
        metadata.get("timeline_marker"),
        "chapter_summary_synthesis.metadata_additions.timeline_marker",
    )
}
