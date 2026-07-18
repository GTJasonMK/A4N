use super::refs::resolve_ref;
use crate::scene_profile::{SceneOccurrenceRecordInput, SceneProfileDbError, SceneProfileDbResult};
use a4n_core::models::scene::scene_occurrence;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, Set};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::scene_profile) async fn insert_scene_occurrence_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    drafts: &[SceneOccurrenceRecordInput],
    scene_ref_map: &BTreeMap<String, i64>,
) -> SceneProfileDbResult<BTreeMap<String, i64>>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut refs = BTreeSet::new();
    let mut map = BTreeMap::new();
    for draft in drafts {
        if !refs.insert(draft.occurrence_ref.clone()) {
            return Err(SceneProfileDbError::InvalidArgument(format!(
                "场景同步存在重复 occurrence_ref: {}",
                draft.occurrence_ref
            )));
        }
        let scene_profile_id =
            resolve_ref(scene_ref_map, &draft.scene_ref, "occurrence.scene_ref")?;
        let occurrence = scene_occurrence::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene_profile_id),
            chapter_id: Set(chapter_id),
            chapter_number: Set(chapter_number),
            occurrence_order: Set(draft.occurrence_order),
            timeline_position: Set(draft.timeline_position.clone()),
            summary: Set(draft.summary.clone()),
            mood: Set(draft.mood.clone()),
            weather_or_lighting: Set(draft.weather_or_lighting.clone()),
            source_span_ids_json: Set(draft.source_span_ids_json.clone()),
            evidence_json: Set(draft.evidence_json.clone()),
            characters_json: Set(draft.characters_json.clone()),
            objects_json: Set(draft.objects_json.clone()),
            key_events_json: Set(draft.key_events_json.clone()),
            entry_state_json: Set(draft.entry_state_json.clone()),
            exit_state_json: Set(draft.exit_state_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
        map.insert(draft.occurrence_ref.clone(), occurrence.id);
    }
    Ok(map)
}
