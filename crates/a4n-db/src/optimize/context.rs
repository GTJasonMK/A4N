use super::*;

pub async fn optimization_chapter_by_id<C>(
    db: &C,
    chapter_id: i64,
) -> Result<Option<chapter::Model>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .map_err(OptimizeDbError::Database)
}

pub async fn optimization_context_rows<C>(
    db: &C,
    input: OptimizationContextReadInput,
) -> Result<Option<OptimizationContextRows>, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let Some(current_chapter) = optimization_chapter_by_id(db, input.chapter_id).await? else {
        return Ok(None);
    };

    let current_chapter_outline = match current_chapter.outline_id {
        Some(outline_id) => chapter_outline::Entity::find_by_id(outline_id)
            .one(db)
            .await
            .map_err(OptimizeDbError::Database)?,
        None => chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
            .filter(chapter_outline::Column::ChapterNumber.eq(current_chapter.chapter_number))
            .one(db)
            .await
            .map_err(OptimizeDbError::Database)?,
    };

    let project = if input.read_project_detail {
        novel_project::Entity::find_by_id(input.project_id)
            .one(db)
            .await
            .map_err(OptimizeDbError::Database)?
    } else {
        None
    };

    let blueprint = if input.read_blueprint {
        match novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(input.project_id))
            .one(db)
            .await
            .map_err(OptimizeDbError::Database)?
        {
            Some(blueprint) => {
                let characters = blueprint_character::Entity::find()
                    .filter(blueprint_character::Column::ProjectId.eq(input.project_id))
                    .all(db)
                    .await
                    .map_err(OptimizeDbError::Database)?;
                let relationships = blueprint_relationship::Entity::find()
                    .filter(blueprint_relationship::Column::ProjectId.eq(input.project_id))
                    .all(db)
                    .await
                    .map_err(OptimizeDbError::Database)?;
                Some(OptimizationBlueprintRows {
                    blueprint,
                    characters,
                    relationships,
                })
            }
            None => None,
        }
    } else {
        None
    };

    let part_outlines = if input.read_outlines {
        part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(input.project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .all(db)
            .await
            .map_err(OptimizeDbError::Database)?
    } else {
        Vec::new()
    };

    let chapter_outlines = if input.read_outlines {
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(input.project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .all(db)
            .await
            .map_err(OptimizeDbError::Database)?
    } else {
        Vec::new()
    };

    let character_profiles = if input.read_character_profiles {
        protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(input.project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .all(db)
            .await
            .map_err(OptimizeDbError::Database)?
    } else {
        Vec::new()
    };

    let foreshadowing_index = if input.read_foreshadowing {
        foreshadowing_index::Entity::find()
            .filter(foreshadowing_index::Column::ProjectId.eq(input.project_id))
            .order_by_asc(foreshadowing_index::Column::SetupChapter)
            .all(db)
            .await
            .map_err(OptimizeDbError::Database)?
    } else {
        Vec::new()
    };

    let mut previous_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(input.project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .filter(chapter::Column::ChapterNumber.lt(current_chapter.chapter_number))
        .order_by_desc(chapter::Column::ChapterNumber)
        .limit(3)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)?;
    previous_chapters.reverse();

    let next_chapters = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(input.project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .filter(chapter::Column::ChapterNumber.gt(current_chapter.chapter_number))
        .order_by_asc(chapter::Column::ChapterNumber)
        .limit(2)
        .all(db)
        .await
        .map_err(OptimizeDbError::Database)?;

    Ok(Some(OptimizationContextRows {
        current_chapter,
        current_chapter_outline,
        project,
        blueprint,
        part_outlines,
        chapter_outlines,
        character_profiles,
        foreshadowing_index,
        previous_chapters,
        next_chapters,
    }))
}

pub async fn write_optimization_chapter_patch_content<C>(
    db: &C,
    input: OptimizationChapterPatchWriteInput,
) -> Result<chapter::Model, OptimizeDbError>
where
    C: ConnectionTrait,
{
    let mut active: chapter::ActiveModel = input.chapter.into();
    crate::chapter_content::assign_chapter_content(
        &mut active,
        input.content,
        input.word_count,
        chapter::STATUS_DRAFT,
        None,
        chrono::Utc::now().naive_utc(),
    );
    active.update(db).await.map_err(OptimizeDbError::Database)
}
