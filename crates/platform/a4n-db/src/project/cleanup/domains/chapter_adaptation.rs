use a4n_db_models::models::{
    chapter::{chapter_shooting_script, chapter_visual_plan},
    project::media_authoring_document,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::super::{ProjectChapterAdaptationDeleteScope, ProjectCleanupDbError};
use crate::project::validate_positive_i64;

pub(crate) async fn delete_project_chapter_adaptation_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    _scope: ProjectChapterAdaptationDeleteScope,
) -> Result<(), ProjectCleanupDbError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    // A source reset must not remove work owned by its derived projects.
    chapter_shooting_script::Entity::delete_many()
        .filter(chapter_shooting_script::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    chapter_visual_plan::Entity::delete_many()
        .filter(chapter_visual_plan::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    media_authoring_document::Entity::delete_many()
        .filter(media_authoring_document::Column::ProjectId.eq(project_id))
        .exec(db)
        .await?;
    Ok(())
}
