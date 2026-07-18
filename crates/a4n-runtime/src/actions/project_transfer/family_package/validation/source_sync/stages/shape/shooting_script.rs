use serde_json::{Map, Value};

use super::{
    super::super::json::validate_json_object_keys,
    common::{
        validate_required_nonempty_stage_source_span_ids, validate_required_positive_stage_integer,
        validate_required_stage_index,
    },
};

pub(in super::super::super) fn validate_regenerate_shooting_script_projection_stage_output(
    object: &Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".shooting_script_projection") {
        return Ok(());
    }
    validate_json_object_keys(object, &["scenes"], path, owner_kind, owner_id)?;
    let scenes_value = object.get("scenes").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.scenes 缺失: {owner_kind}_id={owner_id}")
    })?;
    let scenes = scenes_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scenes 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        scenes.len() == 1,
        "项目族导入包 regenerate_scene {path}.scenes 必须且只能包含一个 index=1 拍摄剧本投影场: {owner_kind}_id={}, scene_count={}",
        owner_id,
        scenes.len()
    );
    let scene = scenes.first().and_then(Value::as_object).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scenes[0] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_required_stage_index(
        scene,
        "index",
        &format!("{path}.scenes[0]"),
        owner_kind,
        owner_id,
    )?;
    for field in [
        "shooting_script_id",
        "shooting_script_revision",
        "chapter_number",
        "shooting_scene_index",
    ] {
        validate_required_positive_stage_integer(
            scene,
            field,
            &format!("{path}.scenes[0]"),
            owner_kind,
            owner_id,
        )?;
    }
    let beats = scene.get("beats").and_then(Value::as_array).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scenes[0].beats 必须是非空数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        !beats.is_empty(),
        "项目族导入包 regenerate_scene {path}.scenes[0].beats 不能为空: {owner_kind}_id={owner_id}"
    );
    validate_required_nonempty_stage_source_span_ids(
        scene,
        &format!("{path}.scenes[0]"),
        owner_kind,
        owner_id,
    )
}
