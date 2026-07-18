use super::contracts::{
    InspirationBlueprintRows, InspirationDbError, InspirationProjectContextRows,
};
use super::project::ensure_inspiration_project_exists;
use a4n_core::models::{
    chapter::chapter,
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub async fn inspiration_project_context_rows(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<InspirationProjectContextRows, InspirationDbError> {
    let project = ensure_inspiration_project_exists(db, project_id).await?;

    let blueprint = match novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(InspirationDbError::Database)?
    {
        Some(blueprint) => {
            let characters = blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .all(db)
                .await
                .map_err(InspirationDbError::Database)?;
            let relationships = blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .all(db)
                .await
                .map_err(InspirationDbError::Database)?;
            Some(InspirationBlueprintRows {
                blueprint,
                characters,
                relationships,
            })
        }
        None => None,
    };

    let mut recent_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_desc(chapter::Column::ChapterNumber)
        .limit(6)
        .all(db)
        .await
        .map_err(InspirationDbError::Database)?;
    recent_chapters.reverse();

    Ok(InspirationProjectContextRows {
        project,
        blueprint,
        recent_chapters,
    })
}
