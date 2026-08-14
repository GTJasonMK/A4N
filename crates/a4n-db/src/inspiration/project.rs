use super::contracts::InspirationDbError;
use a4n_core::models::project::novel_project;
use sea_orm::{DatabaseConnection, EntityTrait};

async fn inspiration_project_by_id(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Option<novel_project::Model>, InspirationDbError> {
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(InspirationDbError::Database)
}

pub(super) async fn ensure_inspiration_project_exists(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<novel_project::Model, InspirationDbError> {
    inspiration_project_by_id(db, project_id)
        .await?
        .ok_or_else(|| InspirationDbError::NotFound(format!("项目不存在: {project_id}")))
}
