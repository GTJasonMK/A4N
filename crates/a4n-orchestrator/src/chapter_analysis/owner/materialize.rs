use a4n_core::error::AppError;
use serde_json::Value;

use a4n_core::source_spans::{SourceSpan, SourceSpanIndex};

mod entities;
mod pipeline;
mod values;

pub(super) use values::{
    analysis_materialize_error, reject_materialize_unknown_fields, required_materialize_string,
    validate_required_string_enum, validate_source_span_ids, validate_string_array,
};

pub fn materialize_analysis_source_spans(
    raw: Value,
    source_spans: &[SourceSpan],
) -> Result<Value, AppError> {
    let mut root = raw.as_object().cloned().ok_or_else(|| {
        AppError::InvalidArgument("chapter_analysis 必须返回 JSON 对象".to_string())
    })?;
    let source_span_index = SourceSpanIndex::new(source_spans);
    pipeline::materialize_entity_source_spans(&mut root, &source_span_index)?;
    Ok(Value::Object(root))
}
