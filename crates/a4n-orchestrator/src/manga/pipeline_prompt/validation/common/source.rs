use a4n_core::error::AppError;
use serde_json::{Map, Value};

pub(in crate::manga::pipeline_prompt::validation) use a4n_core::source_spans::SourceSpanIndex;

use super::fields::{reject_manga_unknown_fields, required_manga_enum, required_manga_string};

pub(in crate::manga::pipeline_prompt::validation) fn validate_manga_source_span_id_array(
    value: &Value,
    source_spans: &SourceSpanIndex<'_>,
    path: &str,
) -> Result<(), AppError> {
    let ids = value
        .get("source_span_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    if ids.is_empty() {
        return Err(AppError::InvalidArgument(format!("{path} 不能为空")));
    }
    for (index, id) in ids.iter().enumerate() {
        let source_span_id = id
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("{path}[{index}] 必须是非空字符串"))
            })?;
        source_spans
            .resolve_text(source_span_id)
            .map_err(|err| AppError::InvalidArgument(format!("{path}[{index}] 无效：{err}")))?;
    }
    Ok(())
}

pub(in crate::manga::pipeline_prompt::validation) fn validate_manga_characters_object_shape(
    characters: &Map<String, Value>,
    source_spans: &SourceSpanIndex<'_>,
    path: &str,
) -> Result<(), AppError> {
    for (name, character) in characters {
        let item_path = format!("{path}.{name}");
        reject_manga_unknown_fields(
            character,
            &[
                "name",
                "appearance",
                "personality",
                "role",
                "gender",
                "age_description",
                "source_span_ids",
            ],
            &item_path,
        )?;
        let canonical_name = required_manga_string(character, "name", &item_path)?;
        if canonical_name != name.trim() {
            return Err(AppError::InvalidArgument(format!(
                "{item_path}.name 必须与角色对象 key 一致"
            )));
        }
        required_manga_string(character, "appearance", &item_path)?;
        required_manga_string(character, "personality", &item_path)?;
        required_manga_enum(
            character,
            "role",
            &item_path,
            &[
                "protagonist",
                "antagonist",
                "supporting",
                "minor",
                "background",
            ],
        )?;
        required_manga_enum(
            character,
            "gender",
            &item_path,
            &["male", "female", "unknown"],
        )?;
        required_manga_string(character, "age_description", &item_path)?;
        validate_manga_source_span_id_array(
            character,
            source_spans,
            &format!("{item_path}.source_span_ids"),
        )?;
    }
    Ok(())
}
