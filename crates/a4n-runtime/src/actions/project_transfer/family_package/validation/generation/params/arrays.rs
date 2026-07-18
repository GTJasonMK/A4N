use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub(in super::super) fn required_params_i64_array(
    object: &Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<Vec<i64>, String> {
    let value = object.get(key).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 缺失: generation_spec_id={spec_id}")
    })?;
    let items = value.as_array().ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是数组: generation_spec_id={spec_id}")
    })?;
    if items.is_empty() {
        return Err(format!(
            "项目族导入包 Galgame 生成意图参数 {key} 必须是非空数组: generation_spec_id={spec_id}"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let id = item.as_i64().ok_or_else(|| {
            format!(
                "项目族导入包 Galgame 生成意图参数 {key} 必须是整数数组: generation_spec_id={spec_id}"
            )
        })?;
        if id <= 0 {
            return Err(format!(
                "项目族导入包 Galgame 生成意图参数 {key} 必须是正整数数组: generation_spec_id={spec_id}"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "项目族导入包 Galgame 生成意图参数 {key} 不能包含重复 ID: generation_spec_id={spec_id}, id={id}"
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

pub(in super::super) fn required_params_string_array(
    object: &Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<Vec<String>, String> {
    let value = object.get(key).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 缺失: generation_spec_id={spec_id}")
    })?;
    let items = value.as_array().ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是数组: generation_spec_id={spec_id}")
    })?;
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "项目族导入包 Galgame 生成意图参数 {key} 必须是非空字符串数组: generation_spec_id={spec_id}"
                )
            })?;
        values.push(value.to_string());
    }
    Ok(values)
}
