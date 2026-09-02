use super::contracts::{ChapterGenerationReferenceContextRows, ProjectChapterContextError};
use a4n_core::models::{
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

pub async fn chapter_generation_reference_context_rows(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ChapterGenerationReferenceContextRows, ProjectChapterContextError> {
    let (blueprint, characters, relationships, protagonist_profiles) = futures::join!(
        async {
            novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                .one(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
        async {
            blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .order_by_asc(blueprint_character::Column::Id)
                .all(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
        async {
            blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .order_by_desc(blueprint_relationship::Column::Strength)
                .order_by_asc(blueprint_relationship::Column::Id)
                .all(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
        async {
            protagonist_profile::Entity::find()
                .filter(protagonist_profile::Column::ProjectId.eq(project_id))
                .order_by_asc(protagonist_profile::Column::Id)
                .all(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
    );
    let blueprint = blueprint?;
    let characters = characters?;
    let relationships = relationships?;
    let protagonist_profiles = protagonist_profiles?;

    Ok(ChapterGenerationReferenceContextRows {
        blueprint,
        characters,
        relationships,
        protagonist_profiles,
    })
}
