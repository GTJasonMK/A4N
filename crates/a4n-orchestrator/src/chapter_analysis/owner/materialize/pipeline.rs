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
    materialize_key_event_source_spans(root, source_spans)?;
    materialize_foreshadowing_source_spans(root, source_spans)
}
