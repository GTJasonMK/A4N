use super::super::{ProjectMutationError, required_text, validate_positive_i64};
use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, Set,
};
use serde_json::Value;

pub async fn blueprint_characters_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<blueprint_character::Model>, DbErr>
where
    C: ConnectionTrait,
{
    blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
}

pub async fn blueprint_relationships_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<blueprint_relationship::Model>, DbErr>
where
    C: ConnectionTrait,
{
    blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
}

pub async fn novel_blueprint_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<novel_blueprint::Model>, DbErr>
where
    C: ConnectionTrait,
{
    novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
}

pub async fn blueprint_versions_by_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<blueprint_version::Model>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_desc(blueprint_version::Column::CreatedAt)
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)
}

pub async fn blueprint_version_by_project_version(
    db: &DatabaseConnection,
    project_id: i64,
    version: &str,
) -> Result<Option<blueprint_version::Model>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let version = required_text(version.to_string(), "version")
        .map_err(ProjectMutationError::InvalidArgument)?;
    blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .filter(blueprint_version::Column::Version.eq(version))
        .one(db)
        .await
        .map_err(ProjectMutationError::Database)
}

pub async fn update_blueprint_world_setting(
    db: &DatabaseConnection,
    project_id: i64,
    world_setting: Value,
) -> Result<Option<novel_blueprint::Model>, ProjectMutationError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectMutationError::InvalidArgument)?;
    let Some(blueprint) = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let mut active: novel_blueprint::ActiveModel = blueprint.into();
    active.world_setting = Set(Some(world_setting));
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(db).await?;
    Ok(Some(updated))
}
