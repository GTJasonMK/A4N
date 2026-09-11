use super::contracts::{
    ProjectMutationError, SOURCE_SYNC_CURRENT, SOURCE_SYNC_ROOT_UPDATED, UpdateProjectRecordInput,
};
use super::queries::project_by_id_record;
use super::rules::required_project_title;
use a4n_db_models::models::project::novel_project;
use chrono::{NaiveDateTime, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set,
};

pub async fn update_project_record(
    db: &DatabaseConnection,
    input: UpdateProjectRecordInput,
) -> Result<novel_project::Model, ProjectMutationError> {
    let project = project_by_id_record(db, input.project_id)
        .await?
        .ok_or_else(|| {
            ProjectMutationError::NotFound(format!("项目不存在: {}", input.project_id))
        })?;
    let mut active: novel_project::ActiveModel = project.into();
    if let Some(title) = input.title {
        active.title = Set(required_project_title(&title)?.to_string());
    }
    if let Some(description) = input.description {
        active.description = Set(description.trim().to_string());
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(ProjectMutationError::Database)
}

pub(crate) async fn mark_derived_projects_root_updated_in_record<C>(
    db: &C,
    root_project_id: i64,
    now: NaiveDateTime,
) -> Result<u64, ProjectMutationError>
where
    C: ConnectionTrait,
{
    let result = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::SourceSyncStatus,
            Expr::value(SOURCE_SYNC_ROOT_UPDATED),
        )
        .col_expr(novel_project::Column::UpdatedAt, Expr::value(now))
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .filter(novel_project::Column::SourceSyncStatus.eq(SOURCE_SYNC_CURRENT))
        .exec(db)
        .await
        .map_err(ProjectMutationError::Database)?;
    Ok(result.rows_affected)
}
