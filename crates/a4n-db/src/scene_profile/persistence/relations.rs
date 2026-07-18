use super::refs::resolve_ref;
use crate::scene_profile::{
    SceneEntityStateRecordInput, SceneProfileDbError, SceneProfileDbResult,
    SceneTransitionRecordInput,
};
use a4n_core::models::scene::{scene_entity_state, scene_occurrence, scene_transition};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use std::collections::BTreeMap;

pub(in crate::scene_profile) async fn insert_scene_transition_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    drafts: &[SceneTransitionRecordInput],
    scene_ref_map: &BTreeMap<String, i64>,
    occurrence_ref_map: &BTreeMap<String, i64>,
) -> SceneProfileDbResult<usize>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    for draft in drafts {
        let from_scene_profile_id = draft
            .from_scene_ref
            .as_deref()
            .map(|value| resolve_ref(scene_ref_map, value, "transition.from_scene_ref"))
            .transpose()?;
        let to_scene_profile_id = resolve_ref(
            scene_ref_map,
            &draft.to_scene_ref,
            "transition.to_scene_ref",
        )?;
        let occurrence_id = draft
            .occurrence_ref
            .as_deref()
            .map(|value| resolve_ref(occurrence_ref_map, value, "transition.occurrence_ref"))
            .transpose()?;
        scene_transition::ActiveModel {
            project_id: Set(project_id),
            chapter_id: Set(chapter_id),
            chapter_number: Set(chapter_number),
            from_scene_profile_id: Set(from_scene_profile_id),
            to_scene_profile_id: Set(to_scene_profile_id),
            occurrence_id: Set(occurrence_id),
            transition_order: Set(draft.transition_order),
            movement_type: Set(draft.movement_type.clone()),
            route_description: Set(draft.route_description.clone()),
            time_cost: Set(draft.time_cost.clone()),
            distance_hint: Set(draft.distance_hint.clone()),
            participants_json: Set(draft.participants_json.clone()),
            object_refs_json: Set(draft.object_refs_json.clone()),
            source_span_ids_json: Set(draft.source_span_ids_json.clone()),
            evidence_json: Set(draft.evidence_json.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(drafts.len())
}

pub(in crate::scene_profile) async fn insert_scene_entity_state_records<C>(
    db: &C,
    project_id: i64,
    drafts: &[SceneEntityStateRecordInput],
    scene_ref_map: &BTreeMap<String, i64>,
    occurrence_ref_map: &BTreeMap<String, i64>,
) -> SceneProfileDbResult<usize>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    for draft in drafts {
        let scene_profile_id =
            resolve_ref(scene_ref_map, &draft.scene_ref, "entity_state.scene_ref")?;
        let occurrence_id = resolve_ref(
            occurrence_ref_map,
            &draft.occurrence_ref,
            "entity_state.occurrence_ref",
        )?;
        let occurrence = scene_occurrence::Entity::find_by_id(occurrence_id)
            .one(db)
            .await?
            .ok_or_else(|| {
                SceneProfileDbError::InvalidArgument(format!(
                    "场景出场不存在: occurrence_ref={}",
                    draft.occurrence_ref
                ))
            })?;
        scene_entity_state::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene_profile_id),
            occurrence_id: Set(occurrence_id),
            chapter_number: Set(occurrence.chapter_number),
            entity_type: Set(draft.entity_type.clone()),
            entity_id: Set(draft.entity_id),
            entity_name_snapshot: Set(draft.entity_name_snapshot.clone()),
            state_key: Set(draft.state_key.clone()),
            state_before_json: Set(draft.state_before_json.clone()),
            state_after_json: Set(draft.state_after_json.clone()),
            position_description: Set(draft.position_description.clone()),
            action_description: Set(draft.action_description.clone()),
            source_span_ids_json: Set(draft.source_span_ids_json.clone()),
            evidence_json: Set(draft.evidence_json.clone()),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(drafts.len())
}
