use super::*;

pub async fn list_external_task_waits(
    db: &DatabaseConnection,
    after_run_id: i64,
) -> Result<Vec<(run::Model, thread::Model, execution::Model)>, AgentDbError> {
    let runs = run::Entity::find()
        .filter(run::Column::Id.gt(after_run_id))
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("waiting"))
        .filter(run::Column::WaitingReason.eq("external"))
        .filter(sea_orm::sea_query::Expr::cust("EXISTS (SELECT 1 FROM agent_execution WHERE run_id = agent_run.id AND execution_kind = 'wait' AND status = 'running')"))
        .order_by_asc(run::Column::Id).limit(32).all(db).await?;
    let mut waits = Vec::new();
    for run in runs {
        let owner = thread::Entity::find_by_id(run.thread_id)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("等待任务的 Agent Thread 不存在".into()))?;
        let executions = execution::Entity::find()
            .filter(execution::Column::RunId.eq(run.id))
            .filter(execution::Column::ExecutionKind.eq("wait"))
            .filter(execution::Column::Status.eq("running"))
            .all(db)
            .await?;
        for execution in executions {
            waits.push((run.clone(), owner.clone(), execution));
        }
    }
    Ok(waits)
}

/// Read one wait execution after the worker has been woken. The list query
/// above intentionally only returns runs still waiting; resumed runs need the
/// same durable identity without weakening the execution-kind/status fence.
pub async fn external_task_wait(
    db: &DatabaseConnection,
    run_id: i64,
    execution_id: i64,
) -> Result<Option<(run::Model, thread::Model, execution::Model)>, AgentDbError> {
    let Some(run_row) = run::Entity::find_by_id(run_id).one(db).await? else {
        return Ok(None);
    };
    let Some(execution_row) = execution::Entity::find_by_id(execution_id)
        .one(db)
        .await?
        .filter(|row| {
            row.run_id == run_id && row.execution_kind == "wait" && row.status == "running"
        })
    else {
        return Ok(None);
    };
    let Some(thread_row) = thread::Entity::find_by_id(run_row.thread_id)
        .one(db)
        .await?
    else {
        return Err(AgentDbError::NotFound(
            "等待任务的 Agent Thread 不存在".into(),
        ));
    };
    Ok(Some((run_row, thread_row, execution_row)))
}

/// A stale task notification cannot release an editor wait or a newer wait.
pub async fn wake_external_task_wait(
    db: &DatabaseConnection,
    run_id: i64,
    execution_id: i64,
) -> Result<(), AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let tx = db.begin().await?;
    lock_threads_on(&tx, [identity.thread_id]).await?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let execution = execution::Entity::find_by_id(execution_id).one(&tx).await?;
    if row.is_active
        && row.status == "waiting"
        && row.waiting_reason.as_deref() == Some("external")
        && execution.is_some_and(|e| {
            e.run_id == run_id && e.execution_kind == "wait" && e.status == "running"
        })
    {
        update_run_state_on(&tx, run_id, None, "queued", None, None, None, None).await?;
        refresh_thread_status_on(&tx, row.thread_id).await?;
    }
    tx.commit().await?;
    Ok(())
}
