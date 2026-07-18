use serde_json::{Map, Value};

use super::{
    super::super::json::validate_json_object_keys,
    common::{
        validate_required_nonempty_stage_source_span_ids, validate_required_stage_scene_index,
    },
};

pub(in super::super::super) fn validate_regenerate_scene_single_scene_line_stage_output(
    object: &Map<String, Value>,
    owner_kind: &str,
    owner_id: i64,
    path: &str,
) -> Result<(), String> {
    if !path.ends_with(".script_adaptation") && !path.ends_with(".character_staging") {
        return Ok(());
    }
    validate_json_object_keys(
        object,
        &["scene_lines", "branch_scene_lines"],
        path,
        owner_kind,
        owner_id,
    )?;
    let branch_scene_lines = object
        .get("branch_scene_lines")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.branch_scene_lines 必须是数组: {owner_kind}_id={owner_id}"
            )
        })?;
    ensure_package!(
        branch_scene_lines.is_empty(),
        "项目族导入包 regenerate_scene {path}.branch_scene_lines 必须为空: {owner_kind}_id={owner_id}"
    );
    let scene_lines_value = object.get("scene_lines").ok_or_else(|| {
        format!("项目族导入包 regenerate_scene {path}.scene_lines 缺失: {owner_kind}_id={owner_id}")
    })?;
    let scene_lines = scene_lines_value.as_array().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scene_lines 必须是数组: {owner_kind}_id={owner_id}"
        )
    })?;
    ensure_package!(
        scene_lines.len() == 1,
        "项目族导入包 regenerate_scene {path}.scene_lines 必须且只能包含一个 scene_index=1 场: {owner_kind}_id={}, scene_count={}",
        owner_id,
        scene_lines.len()
    );
    let scene = scene_lines.first().and_then(Value::as_object).ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene {path}.scene_lines[0] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_required_stage_scene_index(
        scene,
        &format!("{path}.scene_lines[0]"),
        owner_kind,
        owner_id,
    )?;
    let lines = scene
        .get("lines")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.scene_lines[0].lines 必须是非空数组: {owner_kind}_id={owner_id}"
            )
        })?;
    ensure_package!(
        !lines.is_empty(),
        "项目族导入包 regenerate_scene {path}.scene_lines[0].lines 必须是非空数组: {owner_kind}_id={}",
        owner_id
    );
    for (line_index, line) in lines.iter().enumerate() {
        let line = line.as_object().ok_or_else(|| {
            format!(
                "项目族导入包 regenerate_scene {path}.scene_lines[0].lines[{line_index}] 必须是 JSON 对象: {owner_kind}_id={owner_id}"
            )
        })?;
        let beat_ref = line
            .get("beat_ref")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                format!(
                    "项目族导入包 regenerate_scene {path}.scene_lines[0].lines[{line_index}].beat_ref 必须是对象: {owner_kind}_id={owner_id}"
                )
            })?;
        validate_required_stage_scene_index(
            beat_ref,
            &format!("{path}.scene_lines[0].lines[{line_index}].beat_ref"),
            owner_kind,
            owner_id,
        )?;
        super::common::validate_required_positive_stage_integer(
            beat_ref,
            "beat_index",
            &format!("{path}.scene_lines[0].lines[{line_index}].beat_ref"),
            owner_kind,
            owner_id,
        )?;
        validate_required_nonempty_stage_source_span_ids(
            line,
            &format!("{path}.scene_lines[0].lines[{line_index}]"),
            owner_kind,
            owner_id,
        )?;
    }
    Ok(())
}
