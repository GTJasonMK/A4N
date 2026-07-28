//! Galgame 视觉生成意图 params_json 纯校验（无 DB、无 Host）。

use std::collections::BTreeSet;

use a4n_core::models::generation::generation_spec;
use serde_json::Value;

use super::GALGAME_CG_IMPORTANCE;

/// 取可选字符串字段。
pub fn json_owned_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
}

/// 取必填 JSON 对象字段。
pub fn required_visual_json_object<'a>(
    value: &'a Value,
    field: &str,
    spec_id: i64,
) -> Result<&'a Value, String> {
    match value.get(field) {
        Some(raw) if raw.is_object() => Ok(raw),
        Some(_) => Err(format!(
            "Galgame 视觉生成意图参数 {field} 必须是对象: spec_id={spec_id}"
        )),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {field} 缺失: spec_id={spec_id}"
        )),
    }
}

/// 取必填非空字符串字段。
pub fn required_visual_json_str<'a>(
    value: &'a Value,
    field: &str,
    path: &str,
    spec_id: i64,
) -> Result<&'a str, String> {
    match value.get(field) {
        Some(raw) => raw
            .as_str()
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| {
                format!("Galgame 视觉生成意图参数 {path} 必须是非空字符串: spec_id={spec_id}")
            }),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {path} 缺失: spec_id={spec_id}"
        )),
    }
}

/// 取必填 i64 字段。
pub fn required_visual_json_i64(
    value: &Value,
    field: &str,
    path: &str,
    spec_id: i64,
) -> Result<i64, String> {
    match value.get(field) {
        Some(raw) => raw.as_i64().ok_or_else(|| {
            format!("Galgame 视觉生成意图参数 {path} 必须是整数: spec_id={spec_id}")
        }),
        None => Err(format!(
            "Galgame 视觉生成意图参数 {path} 缺失: spec_id={spec_id}"
        )),
    }
}

/// 取非空正整数数组（去重）。
pub fn required_visual_json_i64_array(
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

/// 取非空字符串数组。
pub fn required_visual_json_string_array(
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

fn optional_galgame_cg_importance<'a>(
    value: &'a Value,
    label: &str,
) -> Result<Option<&'a str>, String> {
    let Some(raw) = value.get("cg_importance") else {
        return Ok(None);
    };
    if raw.is_null() {
        return Ok(None);
    }
    let importance = raw
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("{label} 必须是非空字符串或 null"))?;
    if GALGAME_CG_IMPORTANCE.contains(&importance) {
        Ok(Some(importance))
    } else {
        Err(format!("{label} 无效: {importance}"))
    }
}

/// 可选 cg_importance 字段校验。
pub fn validate_optional_galgame_cg_importance(value: &Value, spec_id: i64) -> Result<(), String> {
    optional_galgame_cg_importance(
        value,
        &format!("Galgame 视觉生成意图参数 cg_importance: spec_id={spec_id}"),
    )
    .map(|_| ())
}

/// canonical_key 与期望一致。
pub fn validate_galgame_visual_canonical_key(
    spec: &generation_spec::Model,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key =
        required_visual_json_str(&spec.params_json, "canonical_key", "canonical_key", spec.id)?;
    if canonical_key != expected_key {
        return Err(format!(
            "Galgame 视觉生成意图 canonical_key 与目标不一致: spec_id={}, canonical_key={}, expected={}",
            spec.id, canonical_key, expected_key
        ));
    }
    Ok(())
}

/// 字段 path 对应 i64 与期望一致。
pub fn validate_visual_json_i64_equals(
    spec: &generation_spec::Model,
    value: &Value,
    field: &str,
    path: &str,
    expected: i64,
) -> Result<(), String> {
    let actual = required_visual_json_i64(value, field, path, spec.id)?;
    if actual != expected {
        return Err(format!(
            "Galgame 视觉生成意图参数 {path} 与目标不一致: spec_id={}, actual={}, expected={}",
            spec.id, actual, expected
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn required_object_and_str() {
        let value = json!({"scene": {"title": " 大厅 "}});
        let scene = required_visual_json_object(&value, "scene", 1).unwrap();
        assert_eq!(
            required_visual_json_str(scene, "title", "scene.title", 1).unwrap(),
            "大厅"
        );
    }

    #[test]
    fn cg_importance() {
        assert!(
            validate_optional_galgame_cg_importance(&json!({"cg_importance": "normal"}), 1).is_ok()
        );
        assert!(
            validate_optional_galgame_cg_importance(&json!({"cg_importance": "nope"}), 1).is_err()
        );
    }
}
