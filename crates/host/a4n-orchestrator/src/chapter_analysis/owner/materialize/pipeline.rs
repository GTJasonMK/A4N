use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Map;

use super::entities::{
    materialize_foreshadowing_source_spans, materialize_key_event_source_spans,
    materialize_metadata_source_spans,
};

pub(super) fn materialize_entity_source_spans(
    root: &mut Map<String, serde_json::Value>,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    materialize_metadata_source_spans(root, source_spans)?;
    if let Some(value) = root.get_mut("world_facts") {
        let mut facts: Vec<a4n_domain::fact_feedback::WorldFact> =
            serde_json::from_value(value.clone())?;
        for fact in &mut facts {
            fact.evidence = fact
                .source_span_ids
                .iter()
                .map(|id| source_spans.resolve_text(id))
                .collect::<Result<_, _>>()
                .map_err(AppError::InvalidArgument)?;
        }
        *value = serde_json::to_value(facts)?;
    }
    materialize_key_event_source_spans(root, source_spans)?;
    materialize_foreshadowing_source_spans(root, source_spans)
}
