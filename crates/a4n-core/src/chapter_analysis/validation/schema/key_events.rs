use serde_json::{Map, Value};

use super::super::analysis_schema_error;
use super::super::fields::{reject_unknown_fields, required_array, required_string};

pub(super) fn validate_key_events_schema(
    root: &Map<String, Value>,
) -> Result<(), crate::error::AppError> {
    let key_events = required_array(root, "key_events", "chapter_analysis")?;
    for (index, item) in key_events.iter().enumerate() {
        let context = format!("chapter_analysis.key_events[{index}]");
        let item = item
            .as_object()
            .ok_or_else(|| analysis_schema_error(format!("{context} 必须是对象")))?;
        reject_unknown_fields(
            item,
            &["type", "description", "importance", "involved_characters"],
            &context,
        )?;
        required_string(item, "type", &context)?;
        required_string(item, "description", &context)?;
        required_string(item, "importance", &context)?;
        required_array(item, "involved_characters", &context)?;
    }
    Ok(())
}
