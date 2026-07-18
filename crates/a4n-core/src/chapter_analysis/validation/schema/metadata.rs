use serde_json::{Map, Value};

use super::super::fields::{
    reject_unknown_fields, required_array, required_object, required_string_or_null,
};

pub(super) fn validate_metadata_schema(
    root: &Map<String, Value>,
) -> Result<(), crate::error::AppError> {
    let metadata = required_object(root, "metadata", "chapter_analysis")?;
    reject_unknown_fields(
        metadata,
        &[
            "characters",
            "locations",
            "items",
            "tags",
            "tone",
            "timeline_marker",
        ],
        "chapter_analysis.metadata",
    )?;
    for key in ["characters", "locations", "items", "tags"] {
        required_array(metadata, key, "chapter_analysis.metadata")?;
    }
    required_string_or_null(metadata, "tone", "chapter_analysis.metadata")?;
    required_string_or_null(metadata, "timeline_marker", "chapter_analysis.metadata")?;
    Ok(())
}
