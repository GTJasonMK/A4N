use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS;
use a4n_error::AppError;
use serde_json::Value;

use super::super::common::reject_root_unknown_fields;

pub(super) fn validate_root_fields(value: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        value,
        &["foreshadowing", "key_events"],
        CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
    )
}
