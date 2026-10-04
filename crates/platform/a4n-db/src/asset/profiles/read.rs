use a4n_db_models::models::{asset::object_profile, project::novel_project};
use a4n_domain::object_profile::ObjectProfileRecord;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

use crate::asset::AssetDbError;

pub(crate) fn object_profile_record(model: object_profile::Model) -> ObjectProfileRecord {
    ObjectProfileRecord {
        id: model.id,
        project_id: model.project_id,
        ownership_kind: model.ownership_kind,
        name: model.name,
        normalized_name: model.normalized_name,
        aliases_json: model.aliases_json,
        object_type: model.object_type,
        importance: model.importance,
        visual_description: model.visual_description,
        material_and_state: model.material_and_state,
        story_usage: model.story_usage,
        visual_usage: model.visual_usage,
        generation_usage: model.generation_usage,
        location_context: model.location_context,
        first_seen_chapter: model.first_seen_chapter,
        last_seen_chapter: model.last_seen_chapter,
        evidence_json: model.evidence_json,
        image_prompt_seed: model.image_prompt_seed,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub async fn object_profile_records_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<ObjectProfileRecord>, AssetDbError>
where
    C: ConnectionTrait,
{
    object_profiles_by_project(db, project_id)
        .await
        .map(|profiles| profiles.into_iter().map(object_profile_record).collect())
}

pub async fn object_profile_record_by_id<C>(
    db: &C,
    object_profile_id: i64,
) -> Result<ObjectProfileRecord, AssetDbError>
where
    C: ConnectionTrait,
{
    object_profile_by_id(db, object_profile_id)
        .await
        .map(object_profile_record)
}

pub async fn object_profiles_by_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<object_profile::Model>, AssetDbError>
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
    profiles_result.map_err(AssetDbError::Database)
}

pub async fn object_profile_by_id<C>(
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
