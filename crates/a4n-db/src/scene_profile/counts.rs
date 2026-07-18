use a4n_core::models::scene::scene_occurrence;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter,
    QuerySelect,
};
use std::collections::BTreeMap;

use super::{SceneProfileDbError, SceneProfileDbResult};

#[derive(Debug, FromQueryResult)]
struct SceneOccurrenceCountRow {
    scene_profile_id: i64,
    count: i64,
}

pub async fn scene_occurrence_count(
    db: &DatabaseConnection,
    project_id: i64,
    scene_profile_id: i64,
) -> SceneProfileDbResult<u64> {
    validate_scene_occurrence_count_ids(project_id, &[scene_profile_id])?;
    scene_occurrence::Entity::find()
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.eq(scene_profile_id))
        .count(db)
        .await
        .map_err(SceneProfileDbError::Database)
}

pub async fn scene_occurrence_counts_by_profile(
    db: &DatabaseConnection,
    project_id: i64,
    profile_ids: &[i64],
) -> SceneProfileDbResult<BTreeMap<i64, u64>> {
    validate_scene_occurrence_count_ids(project_id, profile_ids)?;
    if profile_ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows = scene_occurrence::Entity::find()
        .select_only()
        .column(scene_occurrence::Column::SceneProfileId)
        .column_as(scene_occurrence::Column::Id.count(), "count")
        .filter(scene_occurrence::Column::ProjectId.eq(project_id))
        .filter(scene_occurrence::Column::SceneProfileId.is_in(profile_ids.iter().copied()))
        .group_by(scene_occurrence::Column::SceneProfileId)
        .into_model::<SceneOccurrenceCountRow>()
        .all(db)
        .await
        .map_err(SceneProfileDbError::Database)?;
    Ok(rows
        .into_iter()
        .map(|row| (row.scene_profile_id, row.count.max(0) as u64))
        .collect())
}

pub(super) fn validate_scene_occurrence_count_ids(
    project_id: i64,
    profile_ids: &[i64],
) -> SceneProfileDbResult<()> {
    if project_id <= 0 {
        return Err(SceneProfileDbError::InvalidArgument(
            "project_id 必须大于 0".to_string(),
        ));
    }
    if let Some(profile_id) = profile_ids.iter().find(|profile_id| **profile_id <= 0) {
        return Err(SceneProfileDbError::InvalidArgument(format!(
            "scene_profile_id 必须大于 0: {profile_id}"
        )));
    }
    Ok(())
}
