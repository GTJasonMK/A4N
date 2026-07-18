use a4n_core::error::AppError;
use serde_json::Value;

mod required;
use required::validate_required_fields;
mod root;
use root::validate_root_fields;

pub fn validate_chapter_foreshadowing_events(value: &Value) -> Result<(), AppError> {
    validate_root_fields(value)?;
    validate_required_fields(value)
}
