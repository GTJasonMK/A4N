use std::collections::BTreeSet;

use a4n_core::models::scene::scene_occurrence;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::super::persistence::{
    delete_chapter_scene_records, ensure_chapter_in, insert_scene_entity_state_records,
    insert_scene_occurrence_records, insert_scene_transition_records,
    refresh_scene_profiles_after_occurrence_change, upsert_scene_profile_records,
};
use super::super::{SceneProfileDbResult, SceneSyncRecordInput, SceneSyncRecordResult};

pub async fn replace_chapter_scene_sync_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    input: SceneSyncRecordInput,
) -> SceneProfileDbResult<SceneSyncRecordResult>
where
    C: ConnectionTrait,
{
    ensure_chapter_in(db, project_id, chapter_id, chapter_number).await?;
    let old_scene_ids = scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
        .all(db)
        .await?
        .into_iter()
        .map(|occurrence| occurrence.scene_profile_id)
        .collect::<BTreeSet<_>>();

    let scene_ref_map = upsert_scene_profile_records(db, project_id, &input.profiles).await?;
    delete_chapter_scene_records(db, project_id, chapter_id).await?;
    let occurrence_ref_map = insert_scene_occurrence_records(
        db,
        project_id,
        chapter_id,
        chapter_number,
        &input.occurrences,
        &scene_ref_map,
    )
    .await?;
    let transition_count = insert_scene_transition_records(
        db,
        project_id,
        chapter_id,
        chapter_number,
        &input.transitions,
        &scene_ref_map,
        &occurrence_ref_map,
    )
    .await?;
    let entity_state_count = insert_scene_entity_state_records(
        db,
        project_id,
        &input.entity_states,
        &scene_ref_map,
        &occurrence_ref_map,
    )
    .await?;

    let mut affected_scene_ids = old_scene_ids;
    affected_scene_ids.extend(scene_ref_map.values().copied());
    refresh_scene_profiles_after_occurrence_change(db, project_id, &affected_scene_ids, false)
        .await?;

    Ok(SceneSyncRecordResult {
        profile_ids: scene_ref_map.values().copied().collect(),
        profile_ref_ids: scene_ref_map,
        occurrence_count: input.occurrences.len(),
        transition_count,
        entity_state_count,
    })
}
