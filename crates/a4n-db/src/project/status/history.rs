use super::{ProjectStatusHistoryInput, ProjectStatusReadError, ProjectStatusWriteError};
use crate::project::{ProjectCleanupDbError, validate_positive_i64};
use a4n_core::models::project::project_status_history;
use a4n_domain::project::{ProjectStatus, ProjectStatusHistorySummary};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

pub async fn insert_project_status_history<C>(
    db: &C,
    input: ProjectStatusHistoryInput,
) -> Result<(), ProjectStatusWriteError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(input.project_id, "project_id")
        .map_err(ProjectStatusWriteError::InvalidArgument)?;
    let reason = canonical_history_text(input.reason, "reason")?;
    let changed_by = canonical_history_text(input.changed_by, "changed_by")?;
    project_status_history::Entity::insert(project_status_history::ActiveModel {
        project_id: Set(input.project_id),
        from_status: Set(input.from_status.as_str().to_string()),
        to_status: Set(input.to_status.as_str().to_string()),
        reason: Set(Some(reason)),
        changed_by: Set(Some(changed_by)),
        changed_at: Set(input.changed_at),
        ..Default::default()
    })
    .exec(db)
    .await?;
    Ok(())
}

pub async fn delete_project_status_history<C>(
    db: &C,
    project_id: i64,
) -> Result<u64, ProjectCleanupDbError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectCleanupDbError::InvalidArgument)?;
    project_status_history::Entity::delete_many()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
        .map_err(|source| ProjectCleanupDbError::Database { source })
}

pub async fn project_status_history(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<ProjectStatusHistorySummary>, ProjectStatusReadError> {
    validate_positive_i64(project_id, "project_id")
        .map_err(ProjectStatusReadError::InvalidArgument)?;
    let history = project_status_history::Entity::find()
        .filter(project_status_history::Column::ProjectId.eq(project_id))
        .order_by_desc(project_status_history::Column::ChangedAt)
        .order_by_desc(project_status_history::Column::Id)
        .all(db)
        .await
        .map_err(|error| ProjectStatusReadError::database("读取项目状态历史", error))?;
    history
        .into_iter()
        .map(project_status_history_summary)
        .collect()
}

pub(super) fn project_status_history_summary(
    item: project_status_history::Model,
) -> Result<ProjectStatusHistorySummary, ProjectStatusReadError> {
    let from_status = ProjectStatus::parse(&item.from_status).ok_or_else(|| {
        ProjectStatusReadError::InvalidArgument(format!(
            "项目状态历史包含未知 from_status: history_id={}, status={}",
            item.id, item.from_status
        ))
    })?;
    let to_status = ProjectStatus::parse(&item.to_status).ok_or_else(|| {
        ProjectStatusReadError::InvalidArgument(format!(
            "项目状态历史包含未知 to_status: history_id={}, status={}",
            item.id, item.to_status
        ))
    })?;
    let reason = canonical_persisted_history_text(item.reason, item.id, "reason")?;
    canonical_persisted_history_text(item.changed_by, item.id, "changed_by")?;
    Ok(ProjectStatusHistorySummary {
        id: item.id,
        from_status,
        to_status,
        reason,
        created_at: item.changed_at.to_string(),
    })
}

fn canonical_history_text(value: String, field: &str) -> Result<String, ProjectStatusWriteError> {
    if value.is_empty() || value.trim() != value {
        Err(ProjectStatusWriteError::InvalidArgument(format!(
            "{field} 必须是非空规范文本"
        )))
    } else {
        Ok(value)
    }
}

fn canonical_persisted_history_text(
    value: Option<String>,
    history_id: i64,
    field: &str,
) -> Result<String, ProjectStatusReadError> {
    let value = value.ok_or_else(|| {
        ProjectStatusReadError::InvalidArgument(format!(
            "项目状态历史缺少 {field}: history_id={history_id}"
        ))
    })?;
    if value.is_empty() || value.trim() != value {
        return Err(ProjectStatusReadError::InvalidArgument(format!(
            "项目状态历史 {field} 非规范: history_id={history_id}"
        )));
    }
    Ok(value)
}
