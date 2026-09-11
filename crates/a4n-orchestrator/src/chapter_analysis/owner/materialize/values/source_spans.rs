use a4n_domain::source_spans::SourceSpanIndex;
use a4n_error::AppError;
use serde_json::Value;

use super::analysis_materialize_error;

pub(in crate::chapter_analysis::owner) fn validate_source_span_ids(
    value: Option<&Value>,
    source_spans: &SourceSpanIndex<'_>,
    path: &str,
) -> Result<(), AppError> {
    let ids = value
        .and_then(Value::as_array)
        .ok_or_else(|| analysis_materialize_error(format!("{path} 必须是 source_span_id 数组")))?;
    if ids.is_empty() {
        return Err(analysis_materialize_error(format!("{path} 不能为空")));
    }
    for (index, id) in ids.iter().enumerate() {
        let id = id
            .as_str()
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .ok_or_else(|| {
                analysis_materialize_error(format!("{path}[{index}] 必须是非空字符串"))
            })?;
        source_spans
            .resolve_text(id)
            .map_err(analysis_materialize_error)?;
    }
    Ok(())
}
