use a4n_error::AppError;
use serde_json::Value;

use a4n_domain::source_spans::{SourceSpan, SourceSpanIndex};

mod entities;
mod events;
mod foreshadowing;
use entities::validate_fact_entity_array;
use events::validate_event_candidates;
use foreshadowing::validate_foreshadowing_candidates;
mod root;
use root::validate_root_fields;

pub fn validate_chapter_fact_extraction(
    facts: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let source_span_index = SourceSpanIndex::new(source_spans);
    validate_root_fields(facts)?;
    for key in ["characters", "locations", "items"] {
        validate_fact_entity_array(facts, key, &source_span_index)?;
    }
    validate_event_candidates(facts, &source_span_index)?;

    validate_foreshadowing_candidates(facts, &source_span_index)?;
    Ok(())
}
