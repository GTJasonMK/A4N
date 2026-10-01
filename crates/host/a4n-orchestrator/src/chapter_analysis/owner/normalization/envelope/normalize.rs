use a4n_domain::chapter::ChapterRecord;
use a4n_error::AppError;
use serde_json::Value;

use super::super::support::analysis_schema_error;

use super::metadata::analysis_meta;

pub fn normalize_analysis(raw: Value, ch: &ChapterRecord) -> Result<Value, AppError> {
    let mut root = raw
        .as_object()
        .cloned()
        .ok_or_else(|| analysis_schema_error("chapter_analysis 必须返回 JSON 对象"))?;
    a4n_chapter_analysis::validate_chapter_analysis_schema(&root)?;

    root.insert("_meta".to_string(), analysis_meta(ch));

    Ok(Value::Object(root))
}
