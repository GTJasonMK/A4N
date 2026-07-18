use a4n_core::models::project::{
    blueprint_character, blueprint_relationship, blueprint_version, novel_blueprint,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::ProjectBlueprintGraph;
use crate::project::duplicate::ProjectDuplicateError;

pub(in crate::project::duplicate::text) async fn load_project_blueprint_graph<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<ProjectBlueprintGraph>, ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    let Some(blueprint) = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(ProjectDuplicateError::Database)?
    else {
        return Ok(None);
    };
    let versions = blueprint_version::Entity::find()
        .filter(blueprint_version::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_version::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    let relationships = blueprint_relationship::Entity::find()
        .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_relationship::Column::Id)
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    Ok(Some(ProjectBlueprintGraph {
        blueprint,
        versions,
        characters,
        relationships,
    }))
}
