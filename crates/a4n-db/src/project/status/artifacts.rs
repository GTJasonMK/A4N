use super::{ProjectStatusArtifacts, ProjectStatusReadError};
use crate::project::validate_positive_i64;
use a4n_core::models::{
    chapter::{chapter, chapter_outline, part_outline},
    project::novel_blueprint,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

pub async fn project_status_artifacts(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<ProjectStatusArtifacts, ProjectStatusReadError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectStatusReadError::InvalidArgument)?;
    let has_blueprint = novel_blueprint::Entity::find()
        .select_only()
        .column(novel_blueprint::Column::Id)
        .filter(novel_blueprint::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目蓝图", error))?
        .is_some();

    let has_part_outline = part_outline::Entity::find()
        .select_only()
        .column(part_outline::Column::Id)
        .filter(part_outline::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目分部大纲", error))?
        .is_some();

    let has_chapter_outline = chapter_outline::Entity::find()
        .select_only()
        .column(chapter_outline::Column::Id)
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目章节大纲", error))?
        .is_some();

    let has_chapter_content = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::Content)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .into_tuple::<String>()
        .all(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目章节正文", error))?
        .into_iter()
        .any(|content| !content.trim().is_empty());

    Ok(ProjectStatusArtifacts {
        has_blueprint,
        has_part_outline,
        has_chapter_outline,
        has_chapter_content,
    })
}
