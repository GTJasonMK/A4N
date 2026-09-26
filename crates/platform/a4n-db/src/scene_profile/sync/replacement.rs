use std::collections::BTreeSet;

use a4n_db_models::models::{
    chapter::chapter,
    scene::{scene_occurrence, scene_profile},
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

use super::super::SceneProfileDbResult;
use super::super::persistence::{
    delete_chapter_scene_records, ensure_chapter_in, insert_scene_entity_state_records,
    insert_scene_occurrence_records, insert_scene_transition_records,
    refresh_scene_profiles_after_occurrence_change, upsert_scene_profile_records,
};
use a4n_domain::scene_profile::{SceneSyncRecordInput, SceneSyncRecordResult};

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
        .select_only()
        .column(scene_occurrence::Column::SceneProfileId)
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();

    let before = scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .all(db)
        .await?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| super::super::SceneProfileDbError::InvalidArgument("章节不存在".into()))?;
    let scene_ref_map = upsert_scene_profile_records(db, project_id, &input.profiles).await?;
    for draft in &input.profiles {
        let id = scene_ref_map[&draft.scene_ref];
        let profile = scene_profile::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| {
                super::super::SceneProfileDbError::InvalidArgument("场景档案不存在".into())
            })?;
        let previous = before
            .iter()
            .find(|item| item.id == id)
            .map(|item| crate::fact_feedback::profile_fact_snapshot(item, "scene_profile"))
            .transpose()
            .map_err(|error| super::super::SceneProfileDbError::InvalidArgument(error.to_string()))?
            .unwrap_or(serde_json::Value::Null);
        let after = crate::fact_feedback::profile_fact_snapshot(&profile, "scene_profile")
            .map_err(|error| {
                super::super::SceneProfileDbError::InvalidArgument(error.to_string())
            })?;
        crate::fact_feedback::record_profile_fact_observation(
            db,
            project_id,
            "scene_profile",
            id,
            previous,
            after,
            std::slice::from_ref(&chapter),
            &draft.input.evidence_json,
        )
        .await
        .map_err(|error| super::super::SceneProfileDbError::InvalidArgument(error.to_string()))?;
    }
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
