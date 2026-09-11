use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Value;

use super::super::values::analysis_materialize_error;

mod planted;
use planted::materialize_planted_entry;

pub(in crate::chapter_analysis::owner) fn materialize_foreshadowing_source_spans(
    root: &mut serde_json::Map<String, Value>,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    let foreshadowing = root
        .get_mut("foreshadowing")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| analysis_materialize_error("chapter_analysis.foreshadowing 必须是对象"))?;
    let Some(planted) = foreshadowing
        .get_mut("planted")
        .and_then(Value::as_array_mut)
    else {
        return Err(analysis_materialize_error(
            "chapter_analysis.foreshadowing.planted 必须是数组",
        ));
    };
    for (index, item) in planted.iter_mut().enumerate() {
        materialize_planted_entry(item, index, source_spans)?;
    }
    Ok(())
}
