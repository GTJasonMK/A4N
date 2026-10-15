use super::projections::{TaskInfoProjectionRow, persisted_task_info_from_projection_row};
use crate::workflow::{PersistedTaskInfo, WorkflowReadError, validate_positive_i64};
use a4n_db_models::models::task::task_queue;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select};

fn task_info_projection() -> Select<task_queue::Entity> {
    task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .column(task_queue::Column::TaskType)
        .column(task_queue::Column::Priority)
        .column(task_queue::Column::Status)
        .column(task_queue::Column::ProjectId)
        .column(task_queue::Column::RootProjectId)
        .column(task_queue::Column::SourceSnapshotId)
        .column(task_queue::Column::TaskScope)
        .column(task_queue::Column::WorkflowRunId)
        .column(task_queue::Column::Progress)
        .column(task_queue::Column::ErrorMessage)
        .column(task_queue::Column::CreatedAt)
        .column(task_queue::Column::StartedAt)
        .column(task_queue::Column::CompletedAt)
}

pub async fn persisted_task_history(
    db: &impl crate::DbExecutor,
    project_id: Option<i64>,
    offset: u64,
    limit: u64,
) -> Result<Vec<PersistedTaskInfo>, WorkflowReadError> {
    if let Some(project_id) = project_id {
        validate_positive_i64(project_id, "project_id")?;
    }
    let mut query = task_info_projection()
        .order_by_desc(task_queue::Column::CreatedAt)
        .offset(offset)
        .limit(limit);
    if let Some(project_id) = project_id {
        query = query.filter(task_queue::Column::ProjectId.eq(project_id));
    }
    query
        .into_model::<TaskInfoProjectionRow>()
        .all(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .into_iter()
        .map(persisted_task_info_from_projection_row)
        .collect()
}

pub async fn persisted_task_by_id(
    db: &impl crate::DbExecutor,
    task_id: &str,
) -> Result<Option<PersistedTaskInfo>, WorkflowReadError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(WorkflowReadError::InvalidArgument(
            "task_id 不能为空".to_string(),
        ));
    }
    task_info_projection()
        .filter(task_queue::Column::Id.eq(task_id.to_string()))
        .into_model::<TaskInfoProjectionRow>()
        .one(db)
        .await
        .map_err(WorkflowReadError::Database)?
        .map(persisted_task_info_from_projection_row)
        .transpose()
}
