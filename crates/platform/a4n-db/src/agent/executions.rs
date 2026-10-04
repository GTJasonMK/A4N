use super::*;
use sea_orm::ConnectionTrait;

pub(crate) async fn create_execution_on<C>(
    db: &C,
    input: AgentExecutionInput,
    run_row: run::Model,
) -> Result<execution::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_execution_status(&input.status)?;
    if input.execution_key.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "execution_key 不能为空".into(),
        ));
    }
    let args_digest = input.arguments_json.as_ref().map(json_sha256).transpose()?;
    if let Some(existing) = execution::Entity::find()
        .filter(execution::Column::RunId.eq(input.run_id))
        .filter(execution::Column::ExecutionKey.eq(&input.execution_key))
        .one(db)
        .await?
    {
        if existing.arguments_sha256 != args_digest
            || existing.execution_kind != input.execution_kind
            || existing.tool_name != input.tool_name
            || existing.effect != input.effect
            || existing.replay_policy != input.replay_policy
            || existing.resource_preconditions_json != input.resource_preconditions_json
        {
            return Err(AgentDbError::Conflict(format!(
                "execution_key 已用于不同操作、参数或执行策略: {}",
                input.execution_key
            )));
        }
        return Ok(existing);
    }
    let sequence = run_row
        .next_execution_sequence
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent execution sequence 溢出".into()))?;
    let timestamp = now();
    let row = execution::ActiveModel {
        run_id: Set(input.run_id),
        sequence: Set(sequence),
        execution_key: Set(input.execution_key),
        execution_kind: Set(input.execution_kind),
        status: Set(input.status),
        tool_name: Set(input.tool_name),
        effect: Set(input.effect),
        replay_policy: Set(input.replay_policy),
        arguments_json: Set(input.arguments_json),
        arguments_sha256: Set(args_digest),
        resource_preconditions_json: Set(input.resource_preconditions_json),
        result_json: Set(None),
        result_summary: Set(None),
        result_refs_json: Set(None),
        error_message: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        started_at: Set(None),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let mut usage = run_row.usage_json.clone();
    if let Some(usage) = usage.as_object_mut() {
        usage.remove("budget_wait");
    }
    let mut active: run::ActiveModel = run_row.into();
    active.usage_json = Set(usage);
    active.next_execution_sequence = Set(sequence);
    active.updated_at = Set(timestamp);
    active.update(db).await?;
    Ok(row)
}

/// Creates an execution while fencing it to the currently-held Run lease.
///
/// The Run row is locked and the token/expiry are checked in the same
/// transaction as the execution insert, so an expired worker cannot enqueue a
/// side effect after another worker has taken over recovery.
pub async fn create_execution_for_run_lease(
    db: &impl crate::DbExecutor,
    input: AgentExecutionInput,
    lease_token: &str,
) -> Result<AgentExecutionRecord, AgentDbError> {
    if input.status != "prepared" {
        return Err(AgentDbError::InvalidArgument(
            "worker 只能创建 prepared Agent execution".into(),
        ));
    }
    let tx = db.begin().await?;
    let admission = budget::reserve_budget_on(&tx, &input).await;
    let run_row = run::Entity::find_by_id(input.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent run 不存在".into()))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    if let Err(error) = admission {
        if matches!(error, AgentDbError::BudgetExhausted(_)) {
            let mut usage = run_row.usage_json.clone();
            usage["budget_wait"] =
                serde_json::to_value(budget::BudgetDemand::for_execution(&input)?)
                    .map_err(|error| AgentDbError::InvalidArgument(error.to_string()))?;
            let mut active: run::ActiveModel = run_row.into();
            active.usage_json = Set(usage);
            active.update(&tx).await?;
            tx.commit().await?;
        }
        return Err(error);
    }
    let row = create_execution_on(&tx, input, run_row).await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Updates an execution while fencing the write to the current Run lease.
///
/// Execution results are durable side-effect evidence. A worker whose lease
/// expired must not be able to overwrite the result after another worker has
/// taken over recovery.
pub async fn update_execution_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    execution_id: i64,
    status: &str,
    result_json: Option<Value>,
    result_summary: Option<String>,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
) -> Result<AgentExecutionRecord, AgentDbError> {
    validate_execution_status(status)?;
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    let row = execution::Entity::find_by_id(execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent execution 不存在: {execution_id}")))?;
    if row.run_id != run_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 不属于 run {run_id}: {execution_id}"
        )));
    }
    if is_execution_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 已终止，不能从 {} 变为 {status}: {execution_id}",
            row.status
        )));
    }
    if row.status == "awaiting_user" && status != "dispatching" {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 获批后必须先进入 dispatching: {execution_id}"
        )));
    }
    let approval_row = approval::Entity::find()
        .filter(approval::Column::ExecutionId.eq(execution_id))
        .lock_exclusive()
        .one(&tx)
        .await?;
    if row.status == "awaiting_user" {
        let approval = approval_row.as_ref().ok_or_else(|| {
            AgentDbError::Conflict(format!("Agent execution 缺少 approval: {execution_id}"))
        })?;
        if approval.status != "approved" {
            return Err(AgentDbError::Conflict(format!(
                "Agent execution 未获批: {execution_id} ({})",
                approval.status
            )));
        }
    }
    // Approval is evidence until the side effect has actually succeeded.
    // Consuming it while entering `dispatching` loses the authorization when
    // a provider fails and makes a retry appear unapproved.
    // Recovering an idempotent execution reuses its consumed grant and keeps
    // the original consumption timestamp; it does not approve a new operation.
    if status == "succeeded"
        && let Some(approval) = approval_row
    {
        if !super::approvals::approval_authorizes_execution(
            &approval.status,
            row.replay_policy.as_deref(),
            &row.status,
        ) {
            return Err(AgentDbError::Conflict(format!(
                "Agent execution approval 不可消费: {execution_id} ({})",
                approval.status
            )));
        }
        if approval.status == "approved" {
            let mut active: approval::ActiveModel = approval.into();
            active.status = Set("consumed".to_string());
            active.consumed_at = Set(Some(timestamp));
            active.update(&tx).await?;
        }
    }
    let terminal = is_execution_terminal(status);
    let mut active: execution::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.result_json = Set(result_json);
    active.result_summary = Set(result_summary);
    active.result_refs_json = Set(result_refs_json);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(terminal.then_some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row.into())
}
