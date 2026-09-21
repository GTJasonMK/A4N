use super::QueueDbError;
use a4n_db_models::models::{
    generation::{generation_task_checkpoint, workflow_run},
    task::task_queue,
};
use a4n_domain::task_queue::PersistTaskQueueRecordInput;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QueryFilter,
    QuerySelect, QueryTrait, Set, TransactionTrait,
};

pub async fn recoverable_active_task_ids(
    db: &DatabaseConnection,
) -> Result<Vec<String>, QueueDbError> {
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
    db: &DatabaseConnection,
    input: PersistTaskQueueRecordInput,
) -> Result<(), QueueDbError> {
    let now = Utc::now().naive_utc();
    let tx = db.begin().await?;
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
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<Vec<String>, QueueDbError> {
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
