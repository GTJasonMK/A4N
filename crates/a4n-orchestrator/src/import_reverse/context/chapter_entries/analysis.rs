use a4n_db_models::models::chapter::chapter;
use serde_json::{Map, Value};

pub(super) fn analysis_metadata_array(analysis: Option<&Value>, key: &str) -> Option<Value> {
    analysis
        .and_then(|v| v.get("metadata"))
        .and_then(|v| v.get(key))
        .filter(|v| v.as_array().is_some_and(|arr| !arr.is_empty()))
        .cloned()
}

pub(super) fn analysis_value_path(analysis: Option<&Value>, path: &[&str]) -> Option<Value> {
    let mut current = analysis?;
    for key in path {
        current = current.get(*key)?;
    }
    if current.is_null() || current.as_array().is_some_and(|arr| arr.is_empty()) {
        None
    } else {
        Some(current.clone())
    }
}

pub(super) fn chapter_analysis_context(ch: &chapter::Model) -> Result<Option<Value>, String> {
    let Some(analysis) = ch.analysis_data.as_ref() else {
        return Ok(None);
    };
    let analysis = analysis
        .as_object()
        .ok_or_else(|| format!("章节 #{} 的 analysis_data 必须是对象", ch.id))?;
    if analysis.is_empty() {
        return Ok(None);
    }
    let mut compact = Map::new();
    for key in [
        "metadata",
        "character_states",
        "foreshadowing",
        "key_events",
    ] {
        if let Some(value) = analysis.get(key).filter(|value| is_meaningful_json(value)) {
            compact.insert(key.to_string(), value.clone());
        }
    }
    if compact.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Object(compact)))
    }
}

fn is_meaningful_json(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => !items.is_empty(),
        Value::Object(fields) => !fields.is_empty(),
        Value::Bool(_) | Value::Number(_) => true,
    }
}
