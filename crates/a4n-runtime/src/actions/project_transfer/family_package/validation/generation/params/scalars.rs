use a4n_core::models::generation::generation_spec;
use serde_json::{Map, Value};

pub(in super::super) fn required_generation_spec_target_id(
    spec: &generation_spec::Model,
) -> Result<i64, String> {
    spec.target_entity_id.ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 生成意图缺少 target_entity_id: generation_spec_id={}",
            spec.id
        )
    })
}

pub(in super::super) fn generation_spec_params_object(
    spec: &generation_spec::Model,
) -> Result<&Map<String, Value>, String> {
    spec.params_json.as_object().ok_or_else(|| {
        format!(
            "项目族导入包生成意图参数必须是 JSON 对象: generation_spec_id={}",
            spec.id
        )
    })
}

pub(in super::super) fn required_params_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<&'a Map<String, Value>, String> {
    object.get(key).and_then(Value::as_object).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是对象: generation_spec_id={spec_id}")
    })
}

pub(in super::super) fn required_params_i64(
    object: &Map<String, Value>,
    path: &str,
    spec_id: i64,
) -> Result<i64, String> {
    let key = path.rsplit('.').next().unwrap_or(path);
    object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {path} 必须是整数: generation_spec_id={spec_id}")
    })
}

pub(in super::super) fn optional_params_i64(
    object: &Map<String, Value>,
    path: &str,
    spec_id: i64,
) -> Result<Option<i64>, String> {
    let key = path.rsplit('.').next().unwrap_or(path);
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_i64().map(Some).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {path} 必须是整数: generation_spec_id={spec_id}")
    })
}

pub(in super::super) fn required_params_string(
    object: &Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<String, String> {
    let value = object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 生成意图参数 {key} 必须是非空字符串: generation_spec_id={spec_id}"
            )
        })?;
    Ok(value.to_string())
}

pub(in super::super) fn optional_params_string(
    object: &Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<Option<String>, String> {
    let Some(value) = object.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let value = value.as_str().ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 生成意图参数 {key} 必须是字符串: generation_spec_id={spec_id}"
        )
    })?;
    let value = value.trim();
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value.to_string()))
    }
}
