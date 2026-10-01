use a4n_db_models::models::{asset::object_profile, project::novel_project};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use super::record::object_profile_record;
use crate::asset::AssetDbError;
use a4n_domain::object_profile::ObjectProfileRecord;

pub async fn object_profiles_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<ObjectProfileRecord>, AssetDbError>
where
    C: ConnectionTrait,
{
    let (project_result, profiles_result) = futures::join!(
        ensure_object_profile_project_exists(db, project_id),
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(object_profile::Column::FirstSeenChapter)
            .order_by_asc(object_profile::Column::Id)
            .all(db),
    );
    project_result?;
    profiles_result
        .map(|profiles| profiles.into_iter().map(object_profile_record).collect())
        .map_err(AssetDbError::Database)
}

pub async fn object_profile_by_id<C>(
    db: &C,
    object_profile_id: i64,
) -> Result<ObjectProfileRecord, AssetDbError>
where
    C: ConnectionTrait,
{
    object_profile::Entity::find_by_id(object_profile_id)
        .one(db)
        .await?
        .ok_or_else(|| AssetDbError::NotFound("物体档案不存在".to_string()))
        .map(object_profile_record)
}

pub(super) async fn object_profile_model_by_id<C>(
    db: &C,
    object_profile_id: i64,
) -> Result<object_profile::Model, AssetDbError>
where
    C: ConnectionTrait,
{
    object_profile::Entity::find_by_id(object_profile_id)
        .one(db)
        .await?
        .ok_or_else(|| AssetDbError::NotFound("物体档案不存在".to_string()))
}

pub async fn object_profile_project_id<C>(
    db: &C,
    object_profile_id: i64,
) -> Result<i64, AssetDbError>
where
    C: ConnectionTrait,
{
    object_profile::Entity::find_by_id(object_profile_id)
        .select_only()
        .column(object_profile::Column::ProjectId)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .ok_or_else(|| AssetDbError::NotFound("物体档案不存在".to_string()))
}

pub(super) async fn object_profile_by_normalized_name<C>(
    db: &C,
    project_id: i64,
    normalized_name: &str,
) -> Result<Option<object_profile::Model>, AssetDbError>
where
    C: ConnectionTrait,
{
    object_profile::Entity::find()
        .filter(object_profile::Column::ProjectId.eq(project_id))
        .filter(object_profile::Column::NormalizedName.eq(normalized_name))
        .one(db)
        .await
        .map_err(AssetDbError::Database)
}

pub(in crate::asset) async fn ensure_object_profile_project_exists<C>(
    db: &C,
    project_id: i64,
) -> Result<(), AssetDbError>
where
    C: ConnectionTrait,
{
    if project_id <= 0 {
        return Err(AssetDbError::InvalidArgument(
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
        .ok_or_else(|| AssetDbError::NotFound(format!("项目不存在: {project_id}")))
}
