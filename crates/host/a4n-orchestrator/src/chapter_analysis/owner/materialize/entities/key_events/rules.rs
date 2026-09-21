use a4n_error::AppError;
use serde_json::Map;

use super::super::super::super::values::validate_required_string_enum;

pub(super) fn validate_key_event_enums(
    object: &Map<String, serde_json::Value>,
    index: usize,
) -> Result<(), AppError> {
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
        &format!("key_events[{index}].type"),
    )?;
    validate_required_string_enum(
        object.get("importance"),
        &["high", "medium", "low"],
        &format!("key_events[{index}].importance"),
    )?;
    Ok(())
}
