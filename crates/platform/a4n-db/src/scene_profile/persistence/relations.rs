use super::refs::resolve_ref;
use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};
use a4n_db_models::models::scene::{scene_entity_state, scene_occurrence, scene_transition};
use a4n_domain::scene_profile::{SceneEntityStateRecordInput, SceneTransitionRecordInput};
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set};
use std::collections::{BTreeMap, HashMap};

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
    if drafts.is_empty() {
        return Ok(0);
    }
    let now = Utc::now().naive_utc();
    let mut rows = Vec::with_capacity(drafts.len());
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
        rows.push(scene_transition::ActiveModel {
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
        });
    }
    scene_transition::Entity::insert_many(rows)
        .exec_without_returning(db)
        .await?;
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
    if drafts.is_empty() {
        return Ok(0);
    }
    let resolved_refs = drafts
        .iter()
        .map(|draft| {
            let scene_profile_id =
                resolve_ref(scene_ref_map, &draft.scene_ref, "entity_state.scene_ref")?;
            let occurrence_id = resolve_ref(
                occurrence_ref_map,
                &draft.occurrence_ref,
                "entity_state.occurrence_ref",
            )?;
            Ok::<(i64, i64), SceneProfileDbError>((scene_profile_id, occurrence_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let occurrence_ids = resolved_refs
        .iter()
        .map(|(_, occurrence_id)| *occurrence_id)
        .collect::<Vec<_>>();
    let occurrences_by_id = scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::Id)
        .column(scene_occurrence::Column::ChapterNumber)
        .filter(scene_occurrence::Column::Id.is_in(occurrence_ids.iter().copied()))
        .into_tuple::<(i64, i32)>()
        .all(db)
        .await?
        .into_iter()
        .collect::<HashMap<_, _>>();
    let now = Utc::now().naive_utc();
    let mut rows = Vec::with_capacity(drafts.len());
    for (draft, (scene_profile_id, occurrence_id)) in drafts.iter().zip(resolved_refs) {
        let chapter_number = occurrences_by_id
            .get(&occurrence_id)
            .copied()
            .ok_or_else(|| {
                SceneProfileDbError::InvalidArgument(format!(
                    "场景出场不存在: occurrence_ref={}",
                    draft.occurrence_ref
                ))
            })?;
        rows.push(scene_entity_state::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene_profile_id),
            occurrence_id: Set(occurrence_id),
            chapter_number: Set(chapter_number),
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
        });
    }
    scene_entity_state::Entity::insert_many(rows)
        .exec_without_returning(db)
        .await?;
    Ok(drafts.len())
}
