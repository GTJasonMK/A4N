use super::super::evidence::resolve_scene_text_anchor_texts;
use super::super::types::SceneProfileSyncRequestContext;
use super::validation::validate_id_in_set;
use crate::scene_profile::{
    SCENE_ENTITY_STATE_KEYS, SCENE_ENTITY_TYPES, SceneEntityStateDraft,
    normalize_scene_entity_state_key_value, normalize_scene_entity_type_value,
};
use a4n_core::json_contract::{
    ensure_object, optional_positive_i64, optional_string, positive_i64_array,
    reject_unknown_fields, required_array, required_object_value, required_string,
};
use serde_json::Value;

pub(super) fn parse_scene_entity_states(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<SceneEntityStateDraft>, String> {
    let entity_states = required_array(root, "entity_states", "root")?;
    let mut drafts = Vec::with_capacity(entity_states.len());
    for (index, state) in entity_states.iter().enumerate() {
        let path = format!("entity_states[{index}]");
        ensure_object(state, &path)?;
        reject_unknown_fields(
            state,
            &path,
            &[
                "scene_ref",
                "occurrence_ref",
                "entity_type",
                "entity_id",
                "state_key",
                "state_before",
                "state_after",
                "position_description",
                "action_description",
                "source_anchor_ids",
            ],
        )?;
        let entity_type = scene_entity_type_string(state, "entity_type", &path)?;
        let entity_id = optional_positive_i64(state, "entity_id", &path)?;
        match entity_type.as_str() {
            "character" => {
                let id = entity_id.ok_or_else(|| {
                    format!("{path}.entity_id 不能为空，character 必须引用已有角色 ID")
                })?;
                validate_id_in_set(id, &context.character_ids, &format!("{path}.entity_id"))?;
            }
            "object" => {
                let id = entity_id.ok_or_else(|| {
                    format!("{path}.entity_id 不能为空，object 必须引用已有物体档案 ID")
                })?;
                validate_id_in_set(id, &context.object_ids, &format!("{path}.entity_id"))?;
            }
            "event" => {
                if entity_id.is_some() {
                    return Err(format!("{path}.entity_id 必须为 null，event 没有档案 ID"));
                }
            }
            _ => unreachable!("validated enum"),
        }
        let action_description = required_string(state, "action_description", &path)?;
        let entity_name_snapshot = derive_entity_name_snapshot(
            &path,
            &entity_type,
            entity_id,
            &action_description,
            context,
        )?;
        let source_anchor_ids = positive_i64_array(state, "source_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.source_anchor_ids"),
            &source_anchor_ids,
            context,
        )?;
        drafts.push(SceneEntityStateDraft {
            scene_ref: required_string(state, "scene_ref", &path)?,
            occurrence_ref: required_string(state, "occurrence_ref", &path)?,
            entity_type,
            entity_id,
            entity_name_snapshot,
            state_key: scene_entity_state_key_string(state, "state_key", &path)?,
            state_before: required_object_value(state, "state_before", &path)?,
            state_after: required_object_value(state, "state_after", &path)?,
            position_description: optional_string(state, "position_description", &path)?,
            action_description,
            source_anchor_ids,
            evidence,
        });
    }
    Ok(drafts)
}

fn scene_entity_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_scene_entity_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_ENTITY_TYPES.join("/")
        )
    })
}

fn scene_entity_state_key_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_scene_entity_state_key_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_ENTITY_STATE_KEYS.join("/")
        )
    })
}

fn derive_entity_name_snapshot(
    path: &str,
    entity_type: &str,
    entity_id: Option<i64>,
    action_description: &str,
    context: &SceneProfileSyncRequestContext,
) -> Result<String, String> {
    match entity_type {
        "character" => {
            let id = entity_id.ok_or_else(|| {
                format!("{path}.entity_id 不能为空，character 必须引用已有角色 ID")
            })?;
            context
                .character_names
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("{path}.entity_id={id} 缺少角色名称快照来源"))
        }
        "object" => {
            let id = entity_id.ok_or_else(|| {
                format!("{path}.entity_id 不能为空，object 必须引用已有物体档案 ID")
            })?;
            context
                .object_names
                .get(&id)
                .cloned()
                .ok_or_else(|| format!("{path}.entity_id={id} 缺少物体名称快照来源"))
        }
        "event" => Ok(action_description.to_string()),
        _ => unreachable!("validated enum"),
    }
}
