use a4n_core::error::AppError;
use a4n_core::source_spans::SourceSpanIndex;
use serde_json::Value;

use super::super::super::materialize::analysis_materialize_error;

mod entry;
use entry::validate_event_candidate;

pub(super) fn validate_event_candidates(
    facts: &Value,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    let events = facts
        .get("event_candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            analysis_materialize_error("chapter_fact_extraction.event_candidates 必须是数组")
        })?;
    for (index, event) in events.iter().enumerate() {
        validate_event_candidate(event, index, source_spans)?;
    }
    Ok(())
}
