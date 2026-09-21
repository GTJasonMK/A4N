use super::super::evidence::resolve_scene_text_anchor_texts;
use super::super::types::SceneProfileSyncRequestContext;
use super::validation::{id_array_value, validate_transition_references};
use crate::scene_profile::{
    SCENE_TRANSITION_MOVEMENT_TYPES, SceneSyncInput, SceneTransitionDraft,
    normalize_movement_type_value,
};
use a4n_domain::json_contract::{
    ensure_object, optional_string, positive_i64_array, reject_unknown_fields, required_array,
    required_string,
};
use serde_json::Value;

pub(in crate::scene_profile::application) fn parse_scene_transitions(
    root: &Value,
    context: &SceneProfileSyncRequestContext,
    known: &SceneSyncInput,
) -> Result<Vec<SceneTransitionDraft>, String> {
    let transitions = required_array(root, "transitions", "root")?;
    let mut drafts = Vec::with_capacity(transitions.len());
    for (index, transition) in transitions.iter().enumerate() {
        let path = format!("transitions[{index}]");
        ensure_object(transition, &path)?;
        reject_unknown_fields(
            transition,
            &path,
            &[
                "from_scene_ref",
                "to_scene_ref",
                "occurrence_ref",
                "movement_type",
                "route_description",
                "time_cost",
                "distance_hint",
                "participants",
                "object_refs",
                "source_anchor_ids",
            ],
        )?;
        let source_anchor_ids = positive_i64_array(transition, "source_anchor_ids", &path, true)?;
        let evidence = resolve_scene_text_anchor_texts(
            &format!("{path}.source_anchor_ids"),
            &source_anchor_ids,
            context,
        )?;
        drafts.push(SceneTransitionDraft {
            from_scene_ref: optional_string(transition, "from_scene_ref", &path)?,
            to_scene_ref: required_string(transition, "to_scene_ref", &path)?,
            occurrence_ref: optional_string(transition, "occurrence_ref", &path)?,
            transition_order: i32::try_from(index + 1)
                .map_err(|_| format!("{path}.transition_order 超出 i32 范围"))?,
            movement_type: movement_type_string(transition, "movement_type", &path)?,
            route_description: required_string(transition, "route_description", &path)?,
            time_cost: optional_string(transition, "time_cost", &path)?,
            distance_hint: optional_string(transition, "distance_hint", &path)?,
            participants: id_array_value(
                transition,
                "participants",
                &path,
                &context.character_ids,
                "character_profile_id",
            )?,
            object_refs: id_array_value(
                transition,
                "object_refs",
                &path,
                &context.object_ids,
                "object_profile_id",
            )?,
            source_anchor_ids,
            evidence,
        });
    }
    validate_transition_references(known, &drafts)?;
    Ok(drafts)
}

fn movement_type_string(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let text = required_string(value, key, path)?;
    normalize_movement_type_value(&text).map_err(|_| {
        format!(
            "{path}.{key} 无效：{}；允许值：{}",
            text,
            SCENE_TRANSITION_MOVEMENT_TYPES.join("/")
        )
    })
}
