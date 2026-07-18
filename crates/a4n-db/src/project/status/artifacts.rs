use super::{ProjectStatusArtifacts, ProjectStatusReadError};
use crate::project::validate_positive_i64;
use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::novel_blueprint,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};

pub async fn project_status_artifacts(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectStatusArtifacts, ProjectStatusReadError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectStatusReadError::InvalidArgument)?;
    let has_blueprint = novel_blueprint::Entity::find()
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目蓝图", error))?
        .is_some();

    let has_part_outline = part_outline::Entity::find()
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .count(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目分部大纲", error))?
        > 0;

    let has_chapter_outline = chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .count(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目章节大纲", error))?
        > 0;

    let has_chapter_content = chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .all(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目章节正文", error))?
        .into_iter()
        .any(|chapter| chapter.has_available_content());

    Ok(ProjectStatusArtifacts {
        has_blueprint,
        has_part_outline,
        has_chapter_outline,
        has_chapter_content,
    })
}
