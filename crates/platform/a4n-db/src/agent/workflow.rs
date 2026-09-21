use super::*;
use a4n_db_models::models::generation::workflow_run;
use a4n_domain::agent::AgentWorkflowOrigin;
use sea_orm::{DatabaseBackend, Statement};

// Only the approved start execution owns a workflow. Reads and waits cannot
// confer ownership, including after the start receipt is lost.
const OWNER_JOIN: &str = "FROM agent_run r JOIN agent_execution e ON e.run_id = r.id
    AND e.tool_name IN ('run_canvas_graph', 'run_canvas_node')
    JOIN workflow_run w ON w.id::text = e.resource_preconditions_json->>'workflow_run_id'";

pub async fn workflow_owner<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<Option<run::Model>, AgentDbError> {
    let rows = run::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            format!("SELECT r.* {OWNER_JOIN} WHERE w.id = $1"),
            [id.into()],
        ))
        .all(db)
        .await?;
    if rows.len() > 1 {
        return Err(AgentDbError::Conflict(
            "Workflow 存在多个 Agent owner".into(),
        ));
    }
    Ok(rows.into_iter().next())
}

pub async fn active_owned_workflows(
    db: &DatabaseConnection,
    after_id: i64,
) -> Result<Vec<workflow_run::Model>, AgentDbError> {
    // `workflow_run` contains json columns, so DISTINCT cannot be applied to
    // its full row on PostgreSQL. EXISTS preserves the owner predicate without
    // multiplying rows when several executions observe the same workflow.
    workflow_run::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT w.*
            FROM workflow_run w
            WHERE w.completed_at IS NULL
              AND w.id > $1
              AND EXISTS (
                  SELECT 1
                  FROM agent_run r
                  JOIN agent_execution e
                    ON e.run_id = r.id
                   AND e.tool_name IN ('run_canvas_graph', 'run_canvas_node')
                  WHERE w.id::text = e.resource_preconditions_json->>'workflow_run_id'
              )
            ORDER BY w.id
            LIMIT 32
            "#,
            [after_id.into()],
        ))
        .all(db)
        .await
        .map_err(AgentDbError::from)
}

pub async fn bind_owned_workflow(
    db: &DatabaseConnection,
    origin: &AgentWorkflowOrigin,
    workflow_id: i64,
) -> Result<(), AgentDbError> {
    let identity = run_by_id(db, origin.run_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let tx = db.begin().await?;
    lock_threads_on(&tx, [identity.thread_id]).await?;
    let owner = lock_runs_on(&tx, [origin.run_id]).await?.remove(0);
    validate_live_run_lease(&owner, &origin.lease_token, now())?;
    generation::ensure_owner_active(&tx, &owner).await?;
    let workflow = workflow_run::Entity::find_by_id(workflow_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Workflow 不存在".into()))?;
    let execution = execution::Entity::find_by_id(origin.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent 启动 Execution 不存在".into()))?;
    let args = execution
        .arguments_json
        .as_ref()
        .and_then(|v| v.get("input"));
    if execution.run_id != owner.id
        || execution.status != "running"
        || !matches!(
            execution.tool_name.as_deref(),
            Some("run_canvas_graph" | "run_canvas_node")
        )
        || workflow.workflow_kind != "video_canvas_graph"
        || workflow.status != "running"
        || workflow.completed_at.is_some()
        || workflow.scope_table.as_deref() != Some("canvas_graph")
        || args
            .and_then(|v| v.get("project_id"))
            .and_then(Value::as_i64)
            != workflow.project_id
        || args.and_then(|v| v.get("graph_id")).and_then(Value::as_i64) != workflow.scope_id
        || (execution.tool_name.as_deref() == Some("run_canvas_node")
            && args.and_then(|v| v.get("node_id"))
                != workflow
                    .input_fingerprint_json
                    .as_ref()
                    .and_then(|v| v.get("node_id")))
    {
        return Err(AgentDbError::Conflict(
            "Workflow 与已批准的 Agent 启动调用不一致".into(),
        ));
    }
    let mut preconditions = execution
        .resource_preconditions_json
        .clone()
        .unwrap_or_else(|| json!({}));
    if let Some(bound) = preconditions.get("workflow_run_id") {
        if bound == &json!(workflow_id) {
            tx.commit().await?;
            return Ok(());
        }
        return Err(AgentDbError::Conflict(
            "Agent 启动调用已绑定其他 Workflow".into(),
        ));
    }
    if workflow_owner(&tx, workflow_id).await?.is_some() {
        return Err(AgentDbError::Conflict("Workflow 已被其他调用绑定".into()));
    }
    preconditions["workflow_run_id"] = json!(workflow_id);
    let mut active: execution::ActiveModel = execution.into();
    active.resource_preconditions_json = Set(Some(preconditions));
    active.updated_at = Set(now());
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn ensure_workflow_running<C: ConnectionTrait>(
    db: &C,
    id: i64,
) -> Result<(), AgentDbError> {
    let row = workflow_run::Entity::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Workflow 不存在".into()))?;
    if row.status != "running" || row.completed_at.is_some() {
        return Err(AgentDbError::Conflict(
            "Workflow 已停止，不能派发新的 Agent 执行".into(),
        ));
    }
    Ok(())
}
