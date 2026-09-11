use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Value;

use super::super::values::analysis_materialize_error;

mod scalars;
use scalars::normalize_metadata_scalars;
mod entities;
use entities::materialize_metadata_entities;

pub(in crate::chapter_analysis::owner) fn materialize_metadata_source_spans(
    root: &mut serde_json::Map<String, Value>,
    source_spans: &SourceSpanIndex<'_>,
) -> Result<(), AppError> {
    let metadata = root
        .get_mut("metadata")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| analysis_materialize_error("chapter_analysis 缺少 metadata 对象"))?;
    materialize_metadata_entities(metadata, source_spans)?;
    normalize_metadata_scalars(metadata)?;
    Ok(())
}
