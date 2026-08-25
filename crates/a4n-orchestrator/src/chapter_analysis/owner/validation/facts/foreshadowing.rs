use a4n_core::error::AppError;
use a4n_core::source_spans::SourceSpanIndex;
use serde_json::Value;

use super::super::super::materialize::analysis_materialize_error;

mod entry;
use entry::validate_foreshadowing_candidate;

pub(super) fn validate_foreshadowing_candidates(
    facts: &Value,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    let candidates = facts
        .get("foreshadowing_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error(
                "chapter_fact_extraction.foreshadowing_candidates 必须是数组",
            )
        })?;
    for (index, item) in candidates.iter().enumerate() {
        validate_foreshadowing_candidate(item, index, source_spans)?;
    }
    Ok(())
}
