use super::super::super::GALGAME_CG_IMPORTANCE;
use super::scalar::{required_visual_json_i64, required_visual_json_str};
use a4n_core::models::generation::generation_spec;
use serde_json::Value;

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

pub(in crate::actions::galgame_records::asset_generation) fn validate_optional_galgame_cg_importance(
    value: &Value,
    spec_id: i64,
) -> Result<(), String> {
    optional_galgame_cg_importance(
        value,
        &format!("Galgame 视觉生成意图参数 cg_importance: spec_id={spec_id}"),
    )
    .map(|_| ())
}

pub(in crate::actions::galgame_records::asset_generation) fn validate_galgame_visual_canonical_key(
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

pub(in crate::actions::galgame_records::asset_generation) fn validate_visual_json_i64_equals(
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
