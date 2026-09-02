use a4n_core::models::{project::novel_project, scene::scene_profile};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};

use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};

pub async fn scene_profiles_by_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> SceneProfileDbResult<Vec<scene_profile::Model>> {
    let (project_result, profiles_result) = futures::join!(
        ensure_project_exists(db, project_id),
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(scene_profile::Column::FirstSeenChapter)
            .order_by_asc(scene_profile::Column::Id)
            .all(db),
    );
    project_result?;
    profiles_result.map_err(SceneProfileDbError::Database)
}

pub async fn scene_profile_by_id(
    db: &DatabaseConnection,
    scene_profile_id: i64,
) -> SceneProfileDbResult<scene_profile::Model> {
    scene_profile::Entity::find_by_id(scene_profile_id)
        .one(db)
        .await?
        .ok_or_else(|| SceneProfileDbError::NotFound("场景档案不存在".to_string()))
}

pub async fn scene_profile_project_id(
    db: &DatabaseConnection,
    scene_profile_id: i64,
) -> SceneProfileDbResult<i64> {
    scene_profile::Entity::find_by_id(scene_profile_id)
        .select_only()
        .column(scene_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| SceneProfileDbError::NotFound("场景档案不存在".to_string()))
}

pub async fn maybe_scene_profile_by_normalized_name<C>(
    db: &C,
    project_id: i64,
    normalized_name: &str,
) -> SceneProfileDbResult<Option<scene_profile::Model>>
where
    C: ConnectionTrait,
{
    scene_profile::Entity::find()
        .filter(scene_profile::Column::ProjectId.eq(project_id))
        .filter(scene_profile::Column::NormalizedName.eq(normalized_name))
        .one(db)
        .await
        .map_err(SceneProfileDbError::Database)
}

pub(super) async fn ensure_project_exists<C>(db: &C, project_id: i64) -> SceneProfileDbResult<()>
where
    C: ConnectionTrait,
{
    if project_id <= 0 {
        return Err(SceneProfileDbError::InvalidArgument(
            "project_id 必须大于 0".to_string(),
        ));
    }
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .map(|_| ())
        .ok_or_else(|| SceneProfileDbError::NotFound(format!("项目不存在: {project_id}")))
}
