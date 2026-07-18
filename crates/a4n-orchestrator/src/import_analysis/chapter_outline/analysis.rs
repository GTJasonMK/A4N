use serde_json::Value;

pub fn analysis_metadata_array(
    analysis: Option<&Value>,
    key: &str,
    chapter_number: i32,
) -> Result<Option<Value>, String> {
    let Some(analysis) = analysis else {
        return Ok(None);
    };
    let Some(metadata) = analysis.get("metadata") else {
        return Ok(None);
    };
    let metadata = metadata
        .as_object()
        .ok_or_else(|| format!("第{chapter_number}章 analysis_data.metadata 必须是对象"))?;
    let Some(value) = metadata.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let values = value
        .as_array()
        .ok_or_else(|| format!("第{chapter_number}章 analysis_data.metadata.{key} 必须是数组"))?;
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Array(values.clone())))
    }
}

pub fn analysis_value_path(
    analysis: Option<&Value>,
    path: &[&str],
    chapter_number: i32,
) -> Result<Option<Value>, String> {
    let Some(mut current) = analysis else {
        return Ok(None);
    };
    for key in path {
        let Some(next) = current.get(*key) else {
            return Ok(None);
        };
        current = next;
    }
    if current.is_null() {
        return Ok(None);
    }
    let values = current.as_array().ok_or_else(|| {
        format!(
            "第{chapter_number}章 analysis_data.{} 必须是数组",
            path.join(".")
        )
    })?;
    if values.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Value::Array(values.clone())))
    }
}
