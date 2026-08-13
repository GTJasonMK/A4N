use super::{PersistTaskQueueRecordInput, QueueDbError};
use a4n_core::models::{generation::generation_task_checkpoint, task::task_queue};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, QueryTrait, Set,
};

pub async fn recoverable_active_task_ids(
    db: &DatabaseConnection,
) -> Result<Vec<String>, QueueDbError> {
    let generation_task_ids = generation_task_checkpoint::Entity::find()
        .select_only()
        .column(generation_task_checkpoint::Column::TaskId)
        .into_query();
    let tasks = task_queue::Entity::find()
        .filter(task_queue::Column::Status.is_in(["pending", "running"]))
        .filter(task_queue::Column::Id.not_in_subquery(generation_task_ids))
        .all(db)
        .await?;
    Ok(tasks.into_iter().map(|task| task.id).collect())
}

pub async fn insert_task_queue_record(
    db: &DatabaseConnection,
    input: PersistTaskQueueRecordInput,
) -> Result<(), QueueDbError> {
    let now = Utc::now().naive_utc();
    task_queue::Entity::insert(task_queue::ActiveModel {
        id: Set(input.task_id),
        task_type: Set(input.task_type),
        priority: Set(input.priority),
        status: Set("pending".to_string()),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        task_scope: Set(input.task_scope),
        workflow_run_id: Set(input.workflow_run_id),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(now),
        started_at: Set(None),
        completed_at: Set(None),
    })
    .exec(db)
    .await?;
    Ok(())
}
