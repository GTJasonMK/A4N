use a4n_db_models::models::galgame::galgame_scene;
use serde_json::Value;
use std::collections::HashSet;

pub(in super::super) fn validate_galgame_scene_entry_kind(
    scene: &galgame_scene::Model,
) -> Result<(), String> {
    match scene.entry_kind.as_str() {
        "main" | "branch" => Ok(()),
        other => Err(format!(
            "项目族导入包 Galgame 场 entry_kind 无效: scene_id={}, entry_kind={other}",
            scene.id
        )),
    }
}

pub(in super::super) fn validate_galgame_participants_json(
    scene_id: i64,
    value: &Value,
    character_profile_ids: &HashSet<i64>,
) -> Result<(), String> {
    let Some(items) = value.as_array() else {
        return Err(format!(
            "项目族导入包 Galgame 场 participants_json 必须是数组: scene_id={scene_id}"
        ));
    };
    for item in items {
        let object = item.as_object().ok_or_else(|| {
            format!("项目族导入包 Galgame 场 participants_json 项必须是对象: scene_id={scene_id}")
        })?;
        let Some(profile_id_value) = object.get("character_profile_id") else {
            continue;
        };
        let Some(profile_id) = profile_id_value.as_i64() else {
            return Err(format!(
                "项目族导入包 Galgame 场 participants_json.character_profile_id 必须是正整数: scene_id={scene_id}"
            ));
        };
        if profile_id <= 0 {
            return Err(format!(
                "项目族导入包 Galgame 场 participants_json.character_profile_id 必须是正整数: scene_id={}, character_profile_id={profile_id}",
                scene_id
            ));
        }
        if !character_profile_ids.contains(&profile_id) {
            return Err(format!(
                "项目族导入包 Galgame 场 participants_json.character_profile_id 不存在: scene_id={}, character_profile_id={profile_id}",
                scene_id
            ));
        }
    }
    Ok(())
}
