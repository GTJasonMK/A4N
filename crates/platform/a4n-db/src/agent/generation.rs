use super::*;
use a4n_db_models::models::generation::generation_task_checkpoint;
use sea_orm::{DatabaseBackend, Statement};

pub async fn generation_owner<C: ConnectionTrait>(
    db: &C,
    task_id: &str,
    attempt: u32,
) -> Result<Option<run::Model>, AgentDbError> {
    let owner_attempt = attempt.to_string();
    let rows = run::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT r.*
            FROM agent_run r
            JOIN agent_thread t ON t.id = r.thread_id
            WHERE EXISTS (
                SELECT 1
                FROM agent_execution e
                WHERE e.run_id = r.id
                  AND e.resource_preconditions_json->>'generation_task_id' = $1
                  AND e.resource_preconditions_json->>'generation_attempt' = $2
            )
            "#
            .to_string(),
            [task_id.into(), owner_attempt.into()],
        ))
        .all(db)
        .await?;
    if rows.len() > 1 {
        return Err(AgentDbError::Conflict(
            "Generation 存在多个 Agent owner".into(),
        ));
    }
    // Validate the attempt even when it has no matching owner association.
    if let Some(task) = generation_task_checkpoint::Entity::find_by_id(task_id)
        .one(db)
        .await?
    {
        if u32::try_from(task.attempt).ok() != Some(attempt) {
            return Err(AgentDbError::Conflict(
                "Generation attempt 已变化，不能脱离原 Agent 预算执行".into(),
            ));
        }
    } else if !rows.is_empty() {
        return Err(AgentDbError::NotFound("Generation task 不存在".into()));
    }
    Ok(rows.into_iter().next())
}

pub async fn active_owned_generation_tasks(
    db: &DatabaseConnection,
    after_task_id: &str,
) -> Result<Vec<generation_task_checkpoint::Model>, AgentDbError> {
    // `generation_task_checkpoint` contains json columns, so DISTINCT cannot
    // be applied to its full row on PostgreSQL. EXISTS keeps the owner
    // predicate while avoiding duplicate rows from the association joins.
    generation_task_checkpoint::Entity::find()
        .from_raw_sql(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            r#"
            SELECT g.*
            FROM generation_task_checkpoint g
            WHERE g.completed_at IS NULL
              AND g.task_id > $1
              AND EXISTS (
                  SELECT 1
                  FROM agent_run r
                  JOIN agent_thread t ON t.id = r.thread_id
                  JOIN agent_execution e ON e.run_id = r.id
                  WHERE e.resource_preconditions_json->>'generation_task_id' = g.task_id
                    AND e.resource_preconditions_json->>'generation_attempt' = g.attempt::text
              )
            ORDER BY g.task_id
            LIMIT 32
            "#,
            [after_task_id.into()],
        ))
        .all(db)
        .await
        .map_err(AgentDbError::from)
}

#[derive(Clone)]
pub enum AgentOwnedTask {
    Generation { task_id: String, attempt: u32 },
    Workflow(i64),
}

impl AgentOwnedTask {
    pub fn execution_key(&self, model: bool, key: &str) -> String {
        let (domain, id) = match self {
            Self::Generation { task_id, .. } => ("optimization", task_id.clone()),
            Self::Workflow(id) => ("canvas", id.to_string()),
        };
        format!(
            "{domain}-{}:{id}:{key}",
            if model { "model" } else { "tool" }
        )
    }

    pub fn preconditions(&self) -> Value {
        match self {
            Self::Generation { task_id, attempt } => {
                json!({"generation_task_id": task_id, "generation_attempt": attempt})
            }
            Self::Workflow(id) => json!({"workflow_run_id": id}),
        }
    }

    async fn owner<C: ConnectionTrait>(&self, db: &C) -> Result<Option<run::Model>, AgentDbError> {
        match self {
            Self::Generation { task_id, attempt } => generation_owner(db, task_id, *attempt).await,
            Self::Workflow(id) => workflow::workflow_owner(db, *id).await,
        }
    }

    async fn ensure_running<C: ConnectionTrait>(&self, db: &C) -> Result<(), AgentDbError> {
        match self {
            Self::Generation { task_id, attempt } => {
                ensure_generation_running(db, task_id, *attempt).await
            }
            Self::Workflow(id) => workflow::ensure_workflow_running(db, *id).await,
        }
    }
}

pub struct AgentTaskExecutionInput {
    pub task: AgentOwnedTask,
    pub execution_key: String,
    pub tool_name: Option<String>,
    pub arguments: Value,
    pub input_message_id: Option<i64>,
    pub base_input_fingerprint: Option<String>,
}

/// Generation owns its execution permit while the Agent Run releases its
/// lease. Fence inner work to the durable association and active Run state.
pub async fn create_task_execution(
    db: &DatabaseConnection,
    input: AgentTaskExecutionInput,
) -> Result<Option<execution::Model>, AgentDbError> {
    let tx = db.begin().await?;
    let Some(owner) = input.task.owner(&tx).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let task = input.task;
    let model = input.tool_name.is_none();
    let mut preconditions = task.preconditions();
    preconditions["input_message_id"] = json!(input.input_message_id);
    preconditions["base_input_fingerprint"] = json!(input.base_input_fingerprint);
    let input = AgentExecutionInput {
        run_id: owner.id,
        execution_key: task.execution_key(model, &input.execution_key),
        execution_kind: if model { "model_call" } else { "tool_call" }.into(),
        status: "prepared".into(),
        effect: Some(
            if input.tool_name.as_deref() == Some("edit_article")
                || (!model && matches!(task, AgentOwnedTask::Workflow(_)))
            {
                "write"
            } else {
                "read"
            }
            .into(),
        ),
        replay_policy: Some(
            if model
                || (input.tool_name.as_deref() != Some("edit_article")
                    && !matches!(task, AgentOwnedTask::Workflow(_)))
            {
                "safe"
            } else {
                "reconcile_required"
            }
            .into(),
        ),
        tool_name: input.tool_name,
        arguments_json: Some(input.arguments),
        resource_preconditions_json: Some(preconditions),
    };
    budget::reserve_budget_on(&tx, &input).await?;
    let owner = run::Entity::find_by_id(owner.id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    ensure_owner_active(&tx, &owner).await?;
    task.ensure_running(&tx).await?;
    let row = create_execution_on(&tx, input, owner.clone()).await?;
    if let Some(message_id) = row
        .resource_preconditions_json
        .as_ref()
        .and_then(|v| v.get("input_message_id"))
        .and_then(Value::as_i64)
    {
        consume_observed_user_inbox_on(&tx, &owner, message_id).await?;
    }
    tx.commit().await?;
    Ok(Some(row))
}

pub async fn update_task_execution(
    db: &DatabaseConnection,
    task: &AgentOwnedTask,
    execution_id: i64,
    result: Option<Value>,
    refs: Option<Value>,
    error: Option<String>,
) -> Result<(), AgentDbError> {
    let tx = db.begin().await?;
    let owner = task
        .owner(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Generation Agent owner 不存在".into()))?;
    let owner = run::Entity::find_by_id(owner.id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    // A late receipt still records real cost after cancellation. Only starting
    // further work requires an active owner and a running Generation attempt.
    if result.is_none() && error.is_none() {
        ensure_owner_active(&tx, &owner).await?;
        task.ensure_running(&tx).await?;
    }
    let row = execution::Entity::find_by_id(execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Generation Agent execution 不存在".into()))?;
    let preconditions = row.resource_preconditions_json.as_ref();
    if row.run_id != owner.id
        || task
            .preconditions()
            .as_object()
            .unwrap()
            .iter()
            .any(|(key, value)| preconditions.and_then(|p| p.get(key)) != Some(value))
    {
        return Err(AgentDbError::Conflict(
            "Generation execution 不属于本任务 attempt".into(),
        ));
    }
    let late_receipt = row.status == "outcome_unknown" && (result.is_some() || error.is_some());
    if is_execution_terminal(&row.status) && !late_receipt {
        return Err(AgentDbError::Conflict(
            "Generation execution 已结束，不能覆盖回执".into(),
        ));
    }
    let status = if error.is_some() {
        "failed"
    } else if result.is_some() {
        "succeeded"
    } else {
        "running"
    };
    let timestamp = now();
    let started_at = row
        .started_at
        .or((status == "running").then_some(timestamp));
    let mut active: execution::ActiveModel = row.into();
    active.status = Set(status.into());
    active.result_json = Set(result);
    active.result_refs_json = Set(refs);
    active.error_message = Set(error);
    active.updated_at = Set(timestamp);
    active.started_at = Set(started_at);
    active.completed_at = Set((status != "running").then_some(timestamp));
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn ensure_owner_active<C: ConnectionTrait>(
    db: &C,
    owner: &run::Model,
) -> Result<(), AgentDbError> {
    if generation_owner_stopped(db, owner).await? {
        return Err(AgentDbError::Conflict(
            "Agent 或祖先 Run 已停止，不能继续生成".into(),
        ));
    }
    Ok(())
}

pub async fn generation_owner_stopped<C: ConnectionTrait>(
    db: &C,
    owner: &run::Model,
) -> Result<bool, AgentDbError> {
    let mut ancestor = Some(owner.clone());
    let mut seen = HashSet::new();
    while let Some(run) = ancestor {
        if !seen.insert(run.id) {
            return Err(AgentDbError::Conflict("Agent lineage 存在环".into()));
        }
        // A completed owner may still send a late receipt, but it cannot start
        // another inner execution; update_task_execution bypasses this guard
        // only when it is recording that receipt.
        if !matches!(
            run.status.as_str(),
            "running" | "queued" | "waiting" | "paused" | "pausing"
        ) || run.waiting_reason.as_deref() == Some("budget")
        {
            return Ok(true);
        }
        ancestor = match run.parent_run_id {
            Some(id) => Some(
                run::Entity::find_by_id(id)
                    .one(db)
                    .await?
                    .ok_or_else(|| AgentDbError::NotFound("Agent ancestor 不存在".into()))?,
            ),
            None => None,
        };
    }
    Ok(budget::run_budget_snapshot(db, owner.id)
        .await?
        .remaining
        .max_duration_seconds
        == Some(0))
}

async fn ensure_generation_running<C: ConnectionTrait>(
    db: &C,
    task_id: &str,
    attempt: u32,
) -> Result<(), AgentDbError> {
    let task = generation_task_checkpoint::Entity::find_by_id(task_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Generation task 不存在".into()))?;
    if u32::try_from(task.attempt).ok() != Some(attempt)
        || task.status != "running"
        || task.completed_at.is_some()
    {
        return Err(AgentDbError::Conflict(
            "Generation attempt 已停止，不能开始新的 Agent 执行".into(),
        ));
    }
    Ok(())
}
