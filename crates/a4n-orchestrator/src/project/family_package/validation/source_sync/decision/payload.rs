use serde_json::Value;

use super::super::contracts::SourceSyncReferenceSets;
use super::super::json::{
    validate_json_id_ref, validate_json_object_keys, validate_optional_json_string_or_null,
    validate_required_json_string_value,
};
use super::result::validate_regenerate_scene_result;

pub(in super::super) fn validate_source_sync_decision_payload(
    owner_kind: &str,
    owner_id: i64,
    strategy: &str,
    payload: &Value,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    if strategy != "regenerate_scene" {
        return Ok(());
    }
    validate_regenerate_scene_payload(owner_kind, owner_id, payload, refs)
}

fn validate_regenerate_scene_payload(
    owner_kind: &str,
    owner_id: i64,
    payload: &Value,
    refs: &SourceSyncReferenceSets,
) -> Result<(), String> {
    let object = payload.as_object().ok_or_else(|| {
        format!(
            "项目族导入包 regenerate_scene 决策载荷必须是 JSON 对象: {owner_kind}_id={owner_id}"
        )
    })?;
    validate_json_object_keys(
        object,
        &["user_action", "galgame_scene_id", "reason", "result"],
        "regenerate_scene 决策载荷",
        owner_kind,
        owner_id,
    )?;
    let expected_galgame_scene_id = validate_json_id_ref(
        object,
        "galgame_scene_id",
        &refs.galgame_scene_ids,
        "Galgame 场",
        owner_kind,
        owner_id,
    )?;
    validate_required_json_string_value(
        object,
        "user_action",
        "regenerate_scene",
        "regenerate_scene 决策载荷",
        owner_kind,
        owner_id,
    )?;
    validate_optional_json_string_or_null(
        object,
        "reason",
        "regenerate_scene 决策载荷",
        owner_kind,
        owner_id,
    )?;
    validate_regenerate_scene_result(
        object.get("result"),
        expected_galgame_scene_id,
        refs,
        owner_kind,
        owner_id,
    )
}
