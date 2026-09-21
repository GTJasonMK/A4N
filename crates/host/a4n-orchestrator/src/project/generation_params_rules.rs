//! 项目族导入包 Galgame 生成意图参数纯校验（无 DB、无 Host）。

use std::collections::{BTreeSet, HashMap};

use a4n_db_models::models::{generation::generation_spec, project::source_snapshot_profile};
use serde_json::{Map, Value};

use crate::galgame::GalgameCgImportance;
use crate::project::snapshot_profile_key;

/// 生成意图缺少 target_entity_id。
pub fn required_generation_spec_target_id(spec: &generation_spec::Model) -> Result<i64, String> {
    spec.target_entity_id.ok_or_else(|| {
        format!(
            "项目族导入包 Galgame 生成意图缺少 target_entity_id: generation_spec_id={}",
            spec.id
        )
    })
}

/// params_json 必须是对象。
pub fn generation_spec_params_object(
    spec: &generation_spec::Model,
) -> Result<&Map<String, Value>, String> {
    spec.params_json.as_object().ok_or_else(|| {
        format!(
            "项目族导入包生成意图参数必须是 JSON 对象: generation_spec_id={}",
            spec.id
        )
    })
}

/// 取嵌套对象参数。
pub fn required_params_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    spec_id: i64,
) -> Result<&'a Map<String, Value>, String> {
    object.get(key).and_then(Value::as_object).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {key} 必须是对象: generation_spec_id={spec_id}")
    })
}

/// 取必填 i64 参数（path 末段为键名）。
pub fn required_params_i64(
    object: &Map<String, Value>,
    path: &str,
    spec_id: i64,
) -> Result<i64, String> {
    let key = path.rsplit('.').next().unwrap_or(path);
    object.get(key).and_then(Value::as_i64).ok_or_else(|| {
        format!("项目族导入包 Galgame 生成意图参数 {path} 必须是整数: generation_spec_id={spec_id}")
    })
}

/// 取可选 i64 参数。
pub fn optional_params_i64(
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

/// 取必填非空字符串参数。
pub fn required_params_string(
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

/// 取非空正整数数组参数（去重）。
pub fn required_params_i64_array(
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

/// 取非空字符串数组参数。
pub fn required_params_string_array(
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

/// 必填 cg_importance 取值校验。
pub fn validate_required_galgame_cg_importance(
    object: &Map<String, Value>,
    spec_id: i64,
) -> Result<(), String> {
    let importance = required_params_string(object, "cg_importance", spec_id)?;
    if GalgameCgImportance::parse(&importance).is_none() {
        return Err(format!(
            "项目族导入包 Galgame 生成意图参数 cg_importance 无效: generation_spec_id={}, cg_importance={}",
            spec_id, importance
        ));
    }
    Ok(())
}

/// canonical_key 与期望一致。
pub fn validate_generation_spec_canonical_key(
    spec: &generation_spec::Model,
    params: &Map<String, Value>,
    expected_key: &str,
) -> Result<(), String> {
    let canonical_key = required_params_string(params, "canonical_key", spec.id)?;
    if canonical_key != expected_key {
        return Err(format!(
            "项目族导入包 Galgame 生成意图 canonical_key 与目标不一致: generation_spec_id={}, canonical_key={}, expected={}",
            spec.id, canonical_key, expected_key
        ));
    }
    Ok(())
}

/// 参数 path 对应 id 与期望一致。
pub fn validate_params_id_equals(
    spec: &generation_spec::Model,
    object: &Map<String, Value>,
    path: &str,
    expected_id: i64,
) -> Result<(), String> {
    let actual_id = required_params_i64(object, path, spec.id)?;
    if actual_id != expected_id {
        return Err(format!(
            "项目族导入包 Galgame 生成意图参数 {path} 与目标不一致: generation_spec_id={}, actual={}, expected={}",
            spec.id, actual_id, expected_id
        ));
    }
    Ok(())
}

/// 角色档案 id 属于源快照。
pub fn validate_snapshot_character_profile_id(
    spec: &generation_spec::Model,
    source_snapshot_id: i64,
    profile_id: i64,
    snapshot_profiles: &HashMap<(i64, String), &source_snapshot_profile::Model>,
) -> Result<(), String> {
    let key = (
        source_snapshot_id,
        snapshot_profile_key("character_profile", profile_id),
    );
    if !snapshot_profiles.contains_key(&key) {
        return Err(format!(
            "项目族导入包 Galgame 生成意图参数角色不属于源快照: generation_spec_id={}, profile_id={}, source_snapshot_id={}",
            spec.id, profile_id, source_snapshot_id
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(value: Value) -> Map<String, Value> {
        value.as_object().cloned().expect("object")
    }

    #[test]
    fn required_params_i64_reads_leaf_key() {
        let object = map(json!({"id": 7}));
        assert_eq!(required_params_i64(&object, "scene.id", 1).unwrap(), 7);
    }

    #[test]
    fn cg_importance_is_required_and_accepts_known_values() {
        let object = map(json!({"cg_importance": "high"}));
        assert!(validate_required_galgame_cg_importance(&object, 1).is_ok());
        assert!(validate_required_galgame_cg_importance(&map(json!({})), 1).is_err());
        let bad = map(json!({"cg_importance": "urgent"}));
        assert!(validate_required_galgame_cg_importance(&bad, 1).is_err());
    }

    #[test]
    fn i64_array_rejects_duplicates() {
        let object = map(json!({"ids": [1, 1]}));
        assert!(required_params_i64_array(&object, "ids", 1).is_err());
    }
}
