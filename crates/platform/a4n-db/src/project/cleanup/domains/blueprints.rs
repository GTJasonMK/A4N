use super::super::{ProjectBlueprintCleanupResult, ProjectCleanupDbError};
use crate::project::validate_positive_i64;
use a4n_db_models::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

pub async fn delete_project_blueprints<C>(
    db: &C,
    project_id: i64,
    delete_versions: bool,
) -> Result<ProjectBlueprintCleanupResult, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    let deleted_blueprint_characters = blueprint_character::Entity::delete_many()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    let deleted_blueprint_relationships = blueprint_relationship::Entity::delete_many()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;
    if delete_versions {
        blueprint_version::Entity::delete_many()
            .filter(blueprint_version::Column::ProjectId.eq(project_id))
            .exec(db)
            .await?;
    }
    let deleted_current_blueprints = novel_blueprint::Entity::delete_many()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?
        .rows_affected;

    Ok(ProjectBlueprintCleanupResult {
        deleted_current_blueprints,
        deleted_blueprint_characters,
        deleted_blueprint_relationships,
    })
}
