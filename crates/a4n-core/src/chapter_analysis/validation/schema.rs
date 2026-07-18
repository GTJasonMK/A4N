mod character_states;
mod foreshadowing;
mod key_events;
mod metadata;
mod summaries;

use super::fields::reject_unknown_fields;
use crate::error::AppError;
use serde_json::{Map, Value};

use character_states::validate_character_states_schema;
use foreshadowing::validate_foreshadowing_schema;
use key_events::validate_key_events_schema;
use metadata::validate_metadata_schema;
use summaries::validate_summaries_schema;

pub(super) fn validate_analysis_schema(root: &Map<String, Value>) -> Result<(), AppError> {
    reject_unknown_fields(
        root,
        &[
            "metadata",
            "summaries",
            "character_states",
            "foreshadowing",
            "key_events",
        ],
        "chapter_analysis",
    )?;

    validate_metadata_schema(root)?;
    validate_summaries_schema(root)?;
    validate_character_states_schema(root)?;
    validate_foreshadowing_schema(root)?;
    validate_key_events_schema(root)?;
    Ok(())
}
