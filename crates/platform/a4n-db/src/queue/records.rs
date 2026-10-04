use super::QueueDbError;
use crate::{DbExecutor, DbHandle};
use a4n_db_models::models::{
    generation::{generation_task_checkpoint, workflow_run},
    task::task_queue,
};
use a4n_domain::task_queue::PersistTaskQueueRecordInput;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, QueryFilter,
    QuerySelect, QueryTrait, Set,
};

#[derive(Debug, thiserror::Error)]
pub(crate) enum TaskWorkflowBindingError {
    #[error("任务已绑定其他 workflow: {current}")]
    Conflict { current: i64 },
    #[error(transparent)]
    Database(#[from] DbErr),
}

pub(crate) async fn bind_locked_task_queue_row_to_workflow<C: ConnectionTrait>(
    db: &C,
    task: task_queue::Model,
    workflow_run_id: i64,
) -> Result<task_queue::Model, TaskWorkflowBindingError> {
    match task.workflow_run_id {
        Some(current) if current == workflow_run_id => Ok(task),
        Some(current) => Err(TaskWorkflowBindingError::Conflict { current }),
        None => {
            let mut active: task_queue::ActiveModel = task.into();
            active.workflow_run_id = Set(Some(workflow_run_id));
            Ok(active.update(db).await?)
        }
    }
}

pub async fn recoverable_active_task_ids(db: &DbHandle) -> Result<Vec<String>, QueueDbError> {
    let db = db.connection();
    let generation_task_ids = generation_task_checkpoint::Entity::find()
        .select_only()
        .column(generation_task_checkpoint::Column::TaskId)
        .into_query();
    let tasks = task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .filter(task_queue::Column::Status.is_in(["pending", "running"]))
        .filter(task_queue::Column::Id.not_in_subquery(generation_task_ids))
        .into_tuple::<String>()
        .all(db)
        .await?;
    Ok(tasks)
}

pub async fn insert_task_queue_record(
    db: &DbHandle,
    input: PersistTaskQueueRecordInput,
) -> Result<(), QueueDbError> {
    let db = db.connection();
    let now = Utc::now().naive_utc();
    let tx = DbExecutor::begin_db_transaction(db).await?;
    if input.task_type == "llm_call"
        && let Some(workflow_run_id) = input.workflow_run_id
    {
        // Serialize admission with workflow termination: cancellation must either
        // reject this call or observe its durable queue row when signalling calls.
        let mut query = workflow_run::Entity::find_by_id(workflow_run_id)
            .select_only()
            .column(workflow_run::Column::Status);
        if tx.get_database_backend() != DbBackend::Sqlite {
            query = query.lock_exclusive();
        }
        let status = query
            .into_tuple::<String>()
            .one(&tx)
            .await?
            .ok_or_else(|| QueueDbError::NotFound(format!("工作流不存在: {workflow_run_id}")))?;
        if status == crate::workflow::WORKFLOW_STATUS_CANCELLED {
            return Err(QueueDbError::Cancelled);
        }
        if crate::workflow::is_terminal_workflow_status(&status) {
            return Err(QueueDbError::InvalidConfig(format!(
                "工作流已结束，不能发起 LLM 调用: {status}"
            )));
        }
    }
    if let Some((package_id, reason_code)) = crate::package::package_admission_block(
        &tx,
        &input.task_type,
        input.task_scope.as_deref(),
        None,
    )
    .await?
    {
        tx.rollback().await?;
        return Err(QueueDbError::AdmissionBlocked {
            package_id,
            reason_code,
        });
    }
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
    .exec(&tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn active_workflow_llm_task_ids(
    db: &DbHandle,
    workflow_run_id: i64,
) -> Result<Vec<String>, QueueDbError> {
    let db = db.connection();
    task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .filter(task_queue::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(task_queue::Column::TaskType.eq("llm_call"))
        .filter(task_queue::Column::Status.is_in(["pending", "running"]))
        .into_tuple::<String>()
        .all(db)
        .await
        .map_err(QueueDbError::Database)
}
