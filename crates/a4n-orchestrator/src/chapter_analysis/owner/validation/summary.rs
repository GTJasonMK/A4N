use a4n_core::{chapter_analysis::CHAPTER_ANALYSIS_STAGE_SUMMARY, error::AppError};
use serde_json::Value;

use super::common::reject_root_unknown_fields;

mod metadata;
use metadata::validate_metadata_additions;
mod fields;
use fields::validate_summaries_fields;

pub fn validate_chapter_summary_synthesis(summary: &Value) -> Result<(), AppError> {
    reject_root_unknown_fields(
        summary,
        &["metadata_additions", "summaries"],
        CHAPTER_ANALYSIS_STAGE_SUMMARY,
    )?;
    validate_metadata_additions(summary.get("metadata_additions"))?;

    validate_summaries_fields(summary.get("summaries"))
}
