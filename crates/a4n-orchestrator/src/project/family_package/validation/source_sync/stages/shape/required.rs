use serde_json::Value;

use super::super::super::json::validate_json_object_keys;

pub(in super::super::super) fn validate_required_regenerate_scene_stage_outputs(
    value: &Value,
    owner_kind: &str,
    owner_id: i64,
) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene result.stage_outputs 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    let stage_keys = [
        "shooting_script_projection",
        "script_adaptation",
        "character_staging",
        "visual_planning",
        "branch_design",
    ];
    validate_json_object_keys(
        object,
        &stage_keys,
        "regenerate_scene result.stage_outputs",
        owner_kind,
        owner_id,
    )?;
    for key in stage_keys {
        let Some(stage) = object.get(key) else {
            return Err(format!(
                "项目族导入包 regenerate_scene result.stage_outputs.{key} 缺失: {owner_kind}_id={owner_id}"
            ));
        };
        if !stage.is_object() {
            return Err(format!(
                "项目族导入包 regenerate_scene result.stage_outputs.{key} 必须是 JSON 对象: {owner_kind}_id={owner_id}"
            ));
        }
    }
    Ok(())
}
