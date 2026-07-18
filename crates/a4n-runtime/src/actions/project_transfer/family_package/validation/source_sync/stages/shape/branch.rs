use serde_json::{Map, Value};

use super::super::super::json::validate_json_object_keys;

pub(in super::super::super) fn validate_regenerate_scene_branch_design_stage_output(
    object: &Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".branch_design") {
        return Ok(());
    }
    validate_json_object_keys(object, &["choices"], path, owner_kind, owner_id)?;
    let choices_value = object.get("choices").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.choices 缺失: {owner_kind}_id={owner_id}")
    })?;
    let choices = choices_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.choices 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        choices.is_empty(),
        "项目族导入包 regenerate_scene {path}.choices 必须为空，局部重跑不允许新增选择支: {owner_kind}_id={}, choices={}",
        owner_id,
        choices.len()
    );
    Ok(())
}
