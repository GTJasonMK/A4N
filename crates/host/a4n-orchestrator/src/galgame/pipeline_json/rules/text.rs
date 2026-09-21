use std::collections::HashSet;

use a4n_error::AppError;

use super::super::contracts::SCRIPT_LINE_TYPES;
use super::invalid;

pub(in crate::galgame::pipeline_json) fn normalize_script_line_type(
    value: String,
    label: &str,
) -> Result<String, AppError> {
    let value = required_text(value, label)?;
    if SCRIPT_LINE_TYPES.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(invalid(format!("{label} 无效: {value}")))
    }
}

pub(in crate::galgame::pipeline_json) fn normalize_source_span_ids(
    values: Vec<String>,
    label: &str,
    require_nonempty: bool,
) -> Result<Vec<String>, AppError> {
    if require_nonempty && values.is_empty() {
        return Err(invalid(format!("{label} 不能为空")));
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = required_text(value, label)?;
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    if require_nonempty && normalized.is_empty() {
        return Err(invalid(format!("{label} 不能为空")));
    }
    Ok(normalized)
}

pub(in crate::galgame::pipeline_json) fn normalize_text_vec(
    values: Vec<String>,
    label: &str,
) -> Result<Vec<String>, AppError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = required_text(value, label)?;
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

pub(in crate::galgame::pipeline_json) fn required_text(
    value: String,
    label: &str,
) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(invalid(format!("{label} 不能为空")))
    } else {
        Ok(value.to_string())
    }
}

pub(in crate::galgame::pipeline_json) fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
