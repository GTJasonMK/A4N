use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, novel_blueprint, novel_project},
};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

use super::{ImportReverseDbError, ReverseBlueprintContextRows, ReverseContextRows};

pub async fn reverse_context_rows<C>(
    db: &C,
    project_id: i64,
    include_blueprint_context: bool,
) -> Result<ReverseContextRows, ImportReverseDbError>
where
    C: ConnectionTrait,
{
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(ImportReverseDbError::Database)?
        .ok_or_else(|| {
            ImportReverseDbError::NotFound(format!("项目不存在: project_id={project_id}"))
        })?;
    let chapters = importable_chapters(db, project_id).await?;
    if chapters.is_empty() {
        return Err(ImportReverseDbError::InvalidArgument(
            "项目没有可用于反推的章节正文".to_string(),
        ));
    }
    let chapter_outlines = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await?;
    let part_outlines = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(part_outline::Column::PartNumber)
        .all(db)
        .await?;
    let blueprint_context = if include_blueprint_context {
        Some(reverse_blueprint_context_rows(db, project_id).await?)
    } else {
        None
    };

    Ok(ReverseContextRows {
        project,
        chapters,
        chapter_outlines,
        part_outlines,
        blueprint_context,
    })
}

async fn importable_chapters<C>(db: &C, project_id: i64) -> Result<Vec<chapter::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map(|chapters| {
            chapters
                .into_iter()
                .filter(|ch| !ch.content.trim().is_empty())
                .collect()
        })
}

async fn reverse_blueprint_context_rows<C>(
    db: &C,
    project_id: i64,
) -> Result<ReverseBlueprintContextRows, DbErr>
where
    C: ConnectionTrait,
{
    let foreshadowing = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .order_by_asc(foreshadowing_index::Column::SetupChapter)
        .all(db)
        .await?;
    let existing_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await?;
    let blueprint_characters = blueprint_character::Entity::find()
        .filter(blueprint_character::Column::ProjectId.eq(project_id))
        .order_by_asc(blueprint_character::Column::Id)
        .all(db)
        .await?;
    let protagonist_profiles = protagonist_profile::Entity::find()
        .filter(protagonist_profile::Column::ProjectId.eq(project_id))
        .order_by_asc(protagonist_profile::Column::Id)
        .all(db)
        .await?;

    Ok(ReverseBlueprintContextRows {
        foreshadowing,
        existing_blueprint,
        blueprint_characters,
        protagonist_profiles,
    })
}
