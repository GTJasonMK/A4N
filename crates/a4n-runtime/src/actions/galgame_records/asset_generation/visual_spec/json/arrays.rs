use serde_json::Value;
use std::collections::BTreeSet;

pub(in crate::actions::galgame_records::asset_generation) fn required_visual_json_i64_array(
    value: &Value,
    field: &str,
    spec_id: i64,
) -> Result<Vec<i64>, String> {
    let raw = value
        .get(field)
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"))?;
    let items = raw
        .as_array()
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 必须是数组: spec_id={spec_id}"))?;
    if items.is_empty() {
        return Err(format!(
            "Galgame 视觉生成意图参数 {field} 必须是非空数组: spec_id={spec_id}"
        ));
    }
    let mut seen = BTreeSet::new();
    let mut ids = Vec::with_capacity(items.len());
    for item in items {
        let id = item.as_i64().ok_or_else(|| {
            format!("Galgame 视觉生成意图参数 {field} 必须是整数数组: spec_id={spec_id}")
        })?;
        if id <= 0 {
            return Err(format!(
                "Galgame 视觉生成意图参数 {field} 必须是正整数数组: spec_id={spec_id}"
            ));
        }
        if !seen.insert(id) {
            return Err(format!(
                "Galgame 视觉生成意图参数 {field} 不能包含重复 ID: spec_id={spec_id}, id={id}"
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

pub(in crate::actions::galgame_records::asset_generation) fn required_visual_json_string_array(
    value: &Value,
    field: &str,
    spec_id: i64,
) -> Result<Vec<String>, String> {
    let raw = value
        .get(field)
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"))?;
    let items = raw
        .as_array()
        .ok_or_else(|| format!("Galgame 视觉生成意图参数 {field} 必须是数组: spec_id={spec_id}"))?;
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        let value = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("Galgame 视觉生成意图参数 {field} 必须是非空字符串数组: spec_id={spec_id}")
            })?;
        values.push(value.to_string());
    }
    Ok(values)
}
