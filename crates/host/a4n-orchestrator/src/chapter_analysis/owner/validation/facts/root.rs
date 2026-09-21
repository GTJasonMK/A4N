use a4n_chapter_analysis::CHAPTER_ANALYSIS_STAGE_FACTS;
use a4n_error::AppError;
use serde_json::Value;

use super::super::common::reject_root_unknown_fields;

pub(super) fn validate_root_fields(facts: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        facts,
        &[
            "characters",
            "locations",
            "items",
            "event_candidates",
            "foreshadowing_candidates",
        ],
        CHAPTER_ANALYSIS_STAGE_FACTS,
    )
}
