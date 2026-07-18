use a4n_core::error::AppError;
use a4n_core::source_spans::SourceSpan;
use serde_json::Value;

use super::super::values::analysis_materialize_error;

mod entry;
use entry::materialize_key_event;

pub(in crate::chapter_analysis::owner) fn materialize_key_event_source_spans(
    root: &mut serde_json::Map<String, Value>,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let Some(events) = root.get_mut("key_events").and_then(Value::as_array_mut) else {
        return Err(analysis_materialize_error(
            "chapter_analysis.key_events 必须是数组",
        ));
    };
    for (index, event) in events.iter_mut().enumerate() {
        materialize_key_event(event, index, source_spans)?;
    }
    Ok(())
}
