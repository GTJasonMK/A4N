use super::{ChapterGenerationReadiness, ProjectStatusReadError};
use crate::project::validate_positive_i64;
use a4n_core::models::{chapter::chapter, chapter::chapter_outline, project::novel_blueprint};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub async fn chapter_generation_readiness(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<ChapterGenerationReadiness, ProjectStatusReadError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectStatusReadError::InvalidArgument)?;
    if chapter_number <= 0 {
        return Err(ProjectStatusReadError::InvalidArgument(
            "chapter_number 必须大于 0".to_string(),
        ));
    }
    let has_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目蓝图", error))?
        .is_some();
    if !has_blueprint {
        return Ok(ChapterGenerationReadiness {
            has_blueprint,
            has_chapter_outline: false,
            existing_chapter: None,
            previous_chapter: None,
        });
    }

    let has_chapter_outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取目标章节大纲", error))?
        .is_some();
    if !has_chapter_outline {
        return Ok(ChapterGenerationReadiness {
            has_blueprint,
            has_chapter_outline,
            existing_chapter: None,
            previous_chapter: None,
        });
    }

    let existing_chapter = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取目标章节", error))?;

    let previous_chapter = if chapter_number > 1
        && !existing_chapter
            .as_ref()
            .is_some_and(chapter::Model::has_available_content)
    {
        chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .filter(chapter::Column::ChapterNumber.eq(chapter_number - 1))
            .one(db)
            .await
            .map_err(|error| ProjectStatusReadError::database("读取前一章节", error))?
    } else {
        None
    };

    Ok(ChapterGenerationReadiness {
        has_blueprint,
        has_chapter_outline,
        existing_chapter,
        previous_chapter,
    })
}
