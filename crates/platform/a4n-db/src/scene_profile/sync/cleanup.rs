use std::collections::BTreeSet;

use a4n_db_models::models::scene::{scene_occurrence, scene_transition};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
};

use super::super::persistence::{
    delete_chapter_scene_records, refresh_scene_profiles_after_occurrence_change,
};
use super::super::{SceneProfileDbError, SceneProfileDbResult};

pub async fn delete_chapter_scene_sync_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prune_ai_orphans: bool,
) -> SceneProfileDbResult<bool>
where
    C: ConnectionTrait,
{
    if project_id <= 0 || chapter_id <= 0 {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景清理 project_id/chapter_id 必须大于 0".to_string(),
        ));
    }
    let old_scene_ids_query = scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::SceneProfileId)
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::ChapterId.eq(chapter_id))
        .into_tuple::<i64>()
        .all(db);
    let old_transition_count_query = scene_transition::Entity::find()
        .filter(scene_transition::Column::ProjectId.eq(project_id))
        .filter(scene_transition::Column::ChapterId.eq(chapter_id))
        .count(db);
    let (old_scene_ids_result, old_transition_count_result) =
        futures::join!(old_scene_ids_query, old_transition_count_query);
    let old_scene_ids = old_scene_ids_result?.into_iter().collect::<BTreeSet<_>>();
    let old_transition_count = old_transition_count_result?;
    if old_scene_ids.is_empty() && old_transition_count == 0 {
        return Ok(false);
    }

    delete_chapter_scene_records(db, project_id, chapter_id).await?;
    refresh_scene_profiles_after_occurrence_change(
        db,
        project_id,
        &old_scene_ids,
        prune_ai_orphans,
    )
    .await?;
    Ok(true)
}
