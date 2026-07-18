use serde_json::{Map, Value};

use super::{
    super::super::json::validate_json_object_keys,
    common::{
        validate_required_nonempty_stage_source_span_ids, validate_required_stage_scene_index,
    },
};

pub(in super::super::super) fn validate_regenerate_scene_visual_planning_stage_output(
    object: &Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".visual_planning") {
        return Ok(());
    }
    validate_json_object_keys(
        object,
        &["backgrounds", "event_cgs"],
        path,
        owner_kind,
        owner_id,
    )?;
    let backgrounds_value = object.get("backgrounds").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.backgrounds 缺失: {owner_kind}_id={owner_id}")
    })?;
    let backgrounds = backgrounds_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.backgrounds 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        backgrounds.len() == 1,
        "项目族导入包 regenerate_scene {path}.backgrounds 必须且只能包含一个 scene_index=1 背景规划: {owner_kind}_id={}, background_count={}",
        owner_id,
        backgrounds.len()
    );
    let background = backgrounds.first().and_then(Value::as_object).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.backgrounds[0] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_required_stage_scene_index(
        background,
        &format!("{path}.backgrounds[0]"),
        owner_kind,
        owner_id,
    )?;
    validate_required_nonempty_stage_source_span_ids(
        background,
        &format!("{path}.backgrounds[0]"),
        owner_kind,
        owner_id,
    )?;
    let event_cgs_value = object.get("event_cgs").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.event_cgs 缺失: {owner_kind}_id={owner_id}")
    })?;
    let event_cgs = event_cgs_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.event_cgs 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    for (index, event_cg) in event_cgs.iter().enumerate() {
        let event_cg = event_cg.as_object().ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.event_cgs[{index}] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
            )
        })?;
        validate_required_stage_scene_index(
            event_cg,
            &format!("{path}.event_cgs[{index}]"),
            owner_kind,
            owner_id,
        )?;
        validate_required_nonempty_stage_source_span_ids(
            event_cg,
            &format!("{path}.event_cgs[{index}]"),
            owner_kind,
            owner_id,
        )?;
    }
    Ok(())
}
