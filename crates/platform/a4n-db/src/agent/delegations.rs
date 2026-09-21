use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;

pub async fn create_delegation<C>(
    db: &C,
    input: AgentDelegationInput,
) -> Result<delegation::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_delegation_status(&input.status)?;
    if input.delegation_key.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "delegation_key 不能为空".into(),
        ));
    }
    if input.capability.trim().is_empty() || input.objective.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "delegation capability/objective 不能为空".into(),
        ));
    }
    if input.parent_thread_id == input.child_thread_id {
        return Err(AgentDbError::InvalidArgument(
            "delegation 不能委派给同一个 thread".into(),
        ));
    }
    if input.child_turn_id.is_some() || input.child_run_id.is_some() {
        return Err(AgentDbError::InvalidArgument(
            "delegation child 必须通过 bind_delegation_child 原子绑定".into(),
        ));
    }
    let timestamp = now();
    delegation::ActiveModel {
        parent_thread_id: Set(input.parent_thread_id),
        parent_run_id: Set(input.parent_run_id),
        parent_execution_id: Set(input.parent_execution_id),
        child_thread_id: Set(input.child_thread_id),
        child_turn_id: Set(input.child_turn_id),
        child_run_id: Set(input.child_run_id),
        delegation_key: Set(input.delegation_key),
        capability: Set(input.capability),
        objective: Set(input.objective),
        success_criteria_json: Set(input.success_criteria_json),
        scope_json: Set(input.scope_json),
        budget_json: Set(input.budget_json),
        status: Set(input.status),
        result_refs_json: Set(None),
        result_summary: Set(None),
        error_message: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::Database)
}

/// Replaying a delegation key is only idempotent when it describes the same
/// business operation.  A reused key with different parameters would
/// otherwise silently return the original child and make the caller believe
/// its new request was accepted.
pub(crate) fn validate_delegation_replay(
    existing: &delegation::Model,
    requested: &AgentDelegationInput,
) -> Result<(), AgentDbError> {
    let matches = existing.parent_thread_id == requested.parent_thread_id
        && existing.parent_run_id == requested.parent_run_id
        && existing.parent_execution_id == requested.parent_execution_id
        && existing.child_thread_id == requested.child_thread_id
        && existing.capability.trim() == requested.capability.trim()
        && existing.objective.trim() == requested.objective.trim()
        && existing.success_criteria_json == requested.success_criteria_json
        && existing.scope_json == requested.scope_json
        && existing.budget_json == requested.budget_json;
    if matches {
        Ok(())
    } else {
        Err(AgentDbError::Conflict(format!(
            "Agent delegation_key 已绑定不同请求: {}",
            requested.delegation_key
        )))
    }
}

pub async fn update_delegation<C>(
    db: &C,
    id: i64,
    status: &str,
    result_refs_json: Option<Value>,
    result_summary: Option<String>,
    error_message: Option<String>,
) -> Result<delegation::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_delegation_status(status)?;
    let row = delegation::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent delegation 不存在: {id}")))?;
    if is_delegation_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 已终止，不能从 {} 变为 {status}: {id}",
            row.status
        )));
    }
    let terminal = is_delegation_terminal(status);
    let timestamp = now();
    let mut active: delegation::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.result_refs_json = Set(result_refs_json);
    active.result_summary = Set(result_summary);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    if terminal {
        active.completed_at = Set(Some(timestamp));
    } else {
        active.completed_at = Set(None);
    }
    active.update(db).await.map_err(AgentDbError::Database)
}

pub async fn bind_delegation_child(
    db: &DatabaseConnection,
    id: i64,
    parent_run_id: i64,
    child_turn_id: i64,
    child_run_id: i64,
) -> Result<delegation::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = bind_delegation_child_on(&tx, id, parent_run_id, child_turn_id, child_run_id).await?;
    tx.commit().await?;
    Ok(row)
}

pub(crate) async fn bind_delegation_child_on<C>(
    db: &C,
    id: i64,
    parent_run_id: i64,
    child_turn_id: i64,
    child_run_id: i64,
) -> Result<delegation::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let parent_run = run::Entity::find_by_id(parent_run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
        })?;
    let row = delegation::Entity::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent delegation 不存在: {id}")))?;
    if row.parent_run_id != parent_run_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 不属于 parent run {parent_run_id}: {id}"
        )));
    }
    if parent_run.thread_id != row.parent_thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent run 与 delegation 绑定不一致: {id}"
        )));
    }
    if row.child_turn_id == Some(child_turn_id) && row.child_run_id == Some(child_run_id) {
        let child_run = run::Entity::find_by_id(child_run_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}"))
            })?;
        let expected_root_run_id = parent_run.root_run_id.unwrap_or(parent_run.id);
        if child_run.thread_id != row.child_thread_id
            || child_run.turn_id != child_turn_id
            || child_run.parent_run_id != Some(parent_run_id)
            || child_run.root_run_id != Some(expected_root_run_id)
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent delegation child lineage 不一致: {id}"
            )));
        }
        return Ok(row);
    }
    if row.child_turn_id.is_some() || row.child_run_id.is_some() {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 已绑定其他 child run: {id}"
        )));
    }
    if row.status != "pending" {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 不能在 {} 状态绑定 child run: {id}",
            row.status
        )));
    }
    if is_run_terminal(&parent_run.status) || !parent_run.is_active {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent run 已终止，不能绑定 child run: {parent_run_id}"
        )));
    }
    let child_turn = turn::Entity::find_by_id(child_turn_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child turn 不存在: {child_turn_id}"))
        })?;
    let child_run = run::Entity::find_by_id(child_run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}")))?;
    if child_turn.thread_id != row.child_thread_id
        || child_run.thread_id != row.child_thread_id
        || child_run.turn_id != child_turn_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child run 与 delegation 绑定不一致: {id}"
        )));
    }
    let expected_root_run_id = parent_run.root_run_id.unwrap_or(parent_run.id);
    match (child_run.parent_run_id, child_run.root_run_id) {
        (None, None) => {}
        (Some(parent_id), Some(root_id))
            if parent_id == parent_run_id && root_id == expected_root_run_id => {}
        _ => {
            return Err(AgentDbError::Conflict(format!(
                "Agent child run 已绑定其他 lineage: {child_run_id}"
            )));
        }
    }
    let timestamp = now();
    let mut child_active: run::ActiveModel = child_run.into();
    child_active.parent_run_id = Set(Some(parent_run_id));
    child_active.root_run_id = Set(Some(expected_root_run_id));
    child_active.updated_at = Set(timestamp);
    child_active.update(db).await?;
    let mut active: delegation::ActiveModel = row.into();
    active.child_turn_id = Set(Some(child_turn_id));
    active.child_run_id = Set(Some(child_run_id));
    active.updated_at = Set(timestamp);
    active.update(db).await.map_err(AgentDbError::Database)
}

/// Creates the delegation and child Turn/Run, then binds lineage before the
/// child can be claimed.  The existing primitives remain the single owners of
/// writer admission and lineage validation.
pub async fn create_child_delegation(
    db: &DatabaseConnection,
    input: AgentChildDelegationInput,
) -> Result<(delegation::Model, turn::Model, run::Model, message::Model), AgentDbError> {
    create_child_delegation_inner(
        db,
        input.delegation,
        input.child_turn,
        input.parent_lease_token,
    )
    .await
}

/// Creates a delegation requested directly by the owning user.  Unlike a
/// model-issued delegation there is no parent worker lease to validate; the
/// locked parent Run is still required to be active and in a state that can
/// wait for a child.  A stale parent worker is fenced by clearing its lease in
/// the same transaction.
pub async fn create_user_delegation(
    db: &DatabaseConnection,
    input: AgentUserDelegationInput,
) -> Result<(delegation::Model, turn::Model, run::Model, message::Model), AgentDbError> {
    create_child_delegation_inner(db, input.delegation, input.child_turn, None).await
}

pub(crate) async fn create_child_delegation_inner(
    db: &DatabaseConnection,
    delegation_input: AgentDelegationInput,
    child_turn_input: AgentTurnInput,
    parent_lease_token: Option<String>,
) -> Result<(delegation::Model, turn::Model, run::Model, message::Model), AgentDbError> {
    if delegation_input.child_thread_id != child_turn_input.thread_id {
        return Err(AgentDbError::Conflict(
            "Agent delegation 与 child turn thread 不一致".into(),
        ));
    }
    if parent_lease_token
        .as_deref()
        .is_some_and(|token| token.trim().is_empty())
    {
        return Err(AgentDbError::InvalidArgument(
            "Agent parent run lease token 不能为空".into(),
        ));
    }
    let input = AgentChildDelegationInput {
        delegation: delegation_input,
        child_turn: child_turn_input,
        parent_lease_token,
    };
    let parent_identity = run::Entity::find_by_id(input.delegation.parent_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                input.delegation.parent_run_id
            ))
        })?;
    if parent_identity.thread_id != input.delegation.parent_thread_id {
        return Err(AgentDbError::Conflict(
            "Agent delegation 与 parent run thread 不一致".into(),
        ));
    }
    // Lock every involved Thread before any Run. This matches ordinary turn
    // admission and prevents cancellation/completion from forming a
    // Thread↔Run deadlock across the parent and canonical child threads.
    let tx = db.begin().await?;
    lock_threads_on(
        &tx,
        [
            input.delegation.parent_thread_id,
            input.delegation.child_thread_id,
        ],
    )
    .await?;
    let parent_run = run::Entity::find_by_id(input.delegation.parent_run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                input.delegation.parent_run_id
            ))
        })?;
    if let Some(existing) = delegation_by_parent_and_key(
        &tx,
        input.delegation.parent_run_id,
        &input.delegation.delegation_key,
    )
    .await?
    {
        validate_delegation_replay(&existing, &input.delegation)?;
        let child_turn_id = existing.child_turn_id.ok_or_else(|| {
            AgentDbError::Conflict(format!(
                "Agent delegation 尚未绑定 child turn: {}",
                existing.id
            ))
        })?;
        let child_run_id = existing.child_run_id.ok_or_else(|| {
            AgentDbError::Conflict(format!(
                "Agent delegation 尚未绑定 child run: {}",
                existing.id
            ))
        })?;
        let child_turn = turn::Entity::find_by_id(child_turn_id)
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!("Agent child turn 不存在: {child_turn_id}"))
            })?;
        let child_run = run::Entity::find_by_id(child_run_id)
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}"))
            })?;
        let child_message = message::Entity::find_by_id(child_turn.input_message_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("Agent child turn 缺少输入消息".into()))?;
        tx.commit().await?;
        return Ok((existing, child_turn, child_run, child_message));
    }
    let timestamp = now();
    if let Some(parent_lease_token) = input.parent_lease_token.as_deref() {
        validate_live_run_lease(&parent_run, parent_lease_token, timestamp)?;
    } else {
        if is_run_terminal(&parent_run.status) || !parent_run.is_active {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent run 已终止，不能创建 user delegation: {}",
                parent_run.id
            )));
        }
        if matches!(
            parent_run.status.as_str(),
            "paused" | "pausing" | "cancelling"
        ) || (parent_run.status == "waiting"
            && parent_run.waiting_reason.as_deref() != Some("child"))
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent run 当前状态不能创建 user delegation: {} ({})",
                parent_run.id, parent_run.status
            )));
        }
    }
    let parent_execution = match input.delegation.parent_execution_id {
        Some(execution_id) => {
            let row = execution::Entity::find_by_id(execution_id)
                .lock_exclusive()
                .one(&tx)
                .await?
                .ok_or_else(|| {
                    AgentDbError::NotFound(format!("Agent parent execution 不存在: {execution_id}"))
                })?;
            if row.run_id != parent_run.id
                || row.status != "prepared"
                || row.execution_kind != "delegation"
            {
                return Err(AgentDbError::Conflict(format!(
                    "Agent parent execution 不可用于委派: {execution_id} ({})",
                    row.status
                )));
            }
            Some(row)
        }
        None => None,
    };
    let delegation = create_delegation(&tx, input.delegation.clone()).await?;
    let (child_turn, child_run, child_message) =
        create_turn_with_run_on(&tx, input.child_turn).await?;
    let child_run_id = child_run.id;
    let delegation = bind_delegation_child_on(
        &tx,
        delegation.id,
        delegation.parent_run_id,
        child_turn.id,
        child_run_id,
    )
    .await?;
    // bind_delegation_child_on updates the child lineage; return the refreshed
    // row so callers never observe a pre-bind parent/root run projection.
    let child_run = run::Entity::find_by_id(child_run_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}")))?;
    let delegation = update_delegation(&tx, delegation.id, "running", None, None, None).await?;
    if let Some(parent_execution) = parent_execution {
        let mut execution_active: execution::ActiveModel = parent_execution.into();
        // A delegation execution represents the whole child lifecycle.  Keep
        // it running until the child returns a terminal result so the parent
        // transcript never exposes a premature tool success.
        execution_active.status = Set("running".to_string());
        execution_active.result_json = Set(Some(json!({
            "delegation_id": delegation.id.to_string(),
            "child_thread_id": delegation.child_thread_id.to_string(),
            "child_turn_id": child_turn.id.to_string(),
            "child_run_id": child_run.id.to_string(),
        })));
        execution_active.result_summary = Set(Some("Child Agent 已启动，等待结果".to_string()));
        execution_active.updated_at = Set(timestamp);
        execution_active.started_at = Set(Some(timestamp));
        execution_active.completed_at = Set(None);
        execution_active.update(&tx).await?;
    }
    let mut parent_active: run::ActiveModel = parent_run.into();
    parent_active.status = Set("waiting".to_string());
    parent_active.waiting_reason = Set(Some("child".to_string()));
    parent_active.worker_id = Set(None);
    parent_active.lease_token = Set(None);
    parent_active.lease_expires_at = Set(None);
    parent_active.updated_at = Set(timestamp);
    parent_active.update(&tx).await?;
    let _ = refresh_thread_status_on(&tx, input.delegation.parent_thread_id).await?;
    let _ = refresh_thread_status_on(&tx, input.delegation.child_thread_id).await?;
    tx.commit().await?;
    Ok((delegation, child_turn, child_run, child_message))
}

/// Completes a child Run, records the delegation result, delivers a durable
/// `child_result` inbox item, and wakes the parent in one transaction.
pub async fn complete_child_delegation(
    db: &DatabaseConnection,
    input: AgentChildCompletionInput,
) -> Result<
    Option<(
        run::Model,
        delegation::Model,
        inbox_model::Model,
        run::Model,
    )>,
    AgentDbError,
> {
    let result_summary = input.result_summary.trim().to_string();
    if result_summary.is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent child result_summary 不能为空".into(),
        ));
    }
    let child_identity = run::Entity::find_by_id(input.child_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let delegation_identity = delegation::Entity::find()
        .filter(delegation::Column::ChildRunId.eq(input.child_run_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent child run 缺少 delegation: {}",
                input.child_run_id
            ))
        })?;
    let parent_identity = run::Entity::find_by_id(delegation_identity.parent_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                delegation_identity.parent_run_id
            ))
        })?;
    if child_identity.thread_id != delegation_identity.child_thread_id
        || parent_identity.thread_id != delegation_identity.parent_thread_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation lineage 不一致: {}",
            delegation_identity.id
        )));
    }
    let tx = db.begin().await?;
    lock_threads_on(&tx, [child_identity.thread_id, parent_identity.thread_id]).await?;
    let locked_runs = lock_runs_on(&tx, [child_identity.id, parent_identity.id]).await?;
    let child_run = locked_runs
        .iter()
        .find(|row| row.id == child_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let parent_run = locked_runs
        .iter()
        .find(|row| row.id == parent_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent parent run 不存在: {}", parent_identity.id))
        })?;
    let timestamp = now();
    validate_live_run_lease(&child_run, &input.child_lease_token, timestamp)?;
    if requeue_for_new_input_on(&tx, &child_run, input.observed_input_message_id).await? {
        tx.commit().await?;
        return Ok(None);
    }
    let child_turn = turn::Entity::find_by_id(child_run.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child turn 不存在: {}", child_run.turn_id))
        })?;
    let mut delegation = lock_delegations_on(&tx, [delegation_identity.id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent delegation 不存在: {}",
                delegation_identity.id
            ))
        })?;
    let parent_execution = if let Some(execution_id) = delegation.parent_execution_id {
        let row = execution::Entity::find_by_id(execution_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!(
                    "Agent parent delegation execution 不存在: {execution_id}"
                ))
            })?;
        if row.run_id != parent_run.id
            || row.execution_kind != "delegation"
            || row.tool_name.as_deref() != Some("delegate")
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent delegation execution 关联不一致: {execution_id}"
            )));
        }
        Some(row)
    } else {
        None
    };
    if delegation.child_run_id != Some(child_run.id)
        || delegation.parent_run_id != parent_run.id
        || delegation.status != "running"
        || child_turn.thread_id != child_run.thread_id
        || input.final_message.thread_id != child_run.thread_id
        || input.final_message.turn_id != Some(child_run.turn_id)
        || input.final_message.run_id != Some(child_run.id)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child completion 关联记录不一致: {}",
            input.child_run_id
        )));
    }
    let final_message = append_message_on(&tx, input.final_message).await?;
    complete_turn(
        &tx,
        child_run.turn_id,
        "completed",
        Some(final_message.id),
        input.result_refs_json.clone(),
        None,
    )
    .await?;
    let mut child_active: run::ActiveModel = child_run.into();
    child_active.status = Set("completed".to_string());
    child_active.waiting_reason = Set(None);
    child_active.is_active = Set(false);
    child_active.worker_id = Set(None);
    child_active.lease_token = Set(None);
    child_active.lease_expires_at = Set(None);
    child_active.updated_at = Set(timestamp);
    child_active.completed_at = Set(Some(timestamp));
    let child_run = child_active.update(&tx).await?;

    let mut delegation_active: delegation::ActiveModel = delegation.clone().into();
    delegation_active.status = Set("awaiting_acceptance".to_string());
    delegation_active.result_refs_json = Set(input.result_refs_json.clone());
    delegation_active.result_summary = Set(Some(result_summary.clone()));
    delegation_active.updated_at = Set(timestamp);
    delegation = delegation_active.update(&tx).await?;

    let parent_tool_message = if let Some(parent_execution) = parent_execution.as_ref() {
        let result = json!({
            "delegation_id": delegation.id.to_string(),
            "child_run_id": child_run.id.to_string(),
            "result_refs": input.result_refs_json.clone(),
            "summary": result_summary.clone(),
        });
        let message = append_parent_delegation_tool_message_on(
            &tx,
            &parent_run,
            &child_run,
            &delegation,
            parent_execution,
            "succeeded",
            result.clone(),
        )
        .await?;
        finish_parent_delegation_execution_on(
            &tx,
            parent_execution.clone(),
            "succeeded",
            result,
            result_summary.clone(),
            input.result_refs_json.clone(),
            None,
            timestamp,
        )
        .await?;
        Some(message)
    } else {
        None
    };

    let mut inbox_payload = json!({
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "result_refs": input.result_refs_json,
    });
    if let Some(message) = parent_tool_message.as_ref() {
        inbox_payload["message_id"] = json!(message.id.to_string());
    }
    let inbox_row = enqueue_inbox(
        &tx,
        AgentInboxInput {
            thread_id: delegation.parent_thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            source_kind: "subagent".to_string(),
            source_thread_id: Some(delegation.child_thread_id),
            inbox_kind: "child_result".to_string(),
            delivery_mode: "followup".to_string(),
            priority: 20,
            dedupe_key: format!("child-result:{}", delegation.id),
            content: result_summary,
            payload_json: Some(inbox_payload),
            available_at: None,
        },
    )
    .await?;
    let parent_run = if parent_run.is_active
        && parent_run.status == "waiting"
        && parent_run.waiting_reason.as_deref() == Some("child")
    {
        let mut parent_active: run::ActiveModel = parent_run.into();
        parent_active.status = Set("queued".to_string());
        parent_active.waiting_reason = Set(None);
        parent_active.worker_id = Set(None);
        parent_active.lease_token = Set(None);
        parent_active.lease_expires_at = Set(None);
        parent_active.next_attempt_at = Set(timestamp);
        parent_active.updated_at = Set(timestamp);
        parent_active.update(&tx).await?
    } else {
        parent_run
    };
    append_run_event_on(
        &tx,
        &child_run,
        "run_completed",
        "completed",
        "Child Agent 已完成",
        None,
        Some(delegation.id),
        Some(final_message.id),
        Some(json!({ "result_summary": delegation.result_summary.clone() })),
    )
    .await?;
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            execution_id: delegation.parent_execution_id,
            delegation_id: Some(delegation.id),
            message_id: parent_tool_message.as_ref().map(|message| message.id),
            actor_kind: "agent".into(),
            actor_id: Some(child_run.id.to_string()),
            kind: "child_completed".into(),
            status: "awaiting_acceptance".into(),
            public_text: "Child Agent 已完成，等待主 Agent 验收".into(),
            payload_json: Some(json!({
                "delegation_id": delegation.id.to_string(),
                "child_run_id": child_run.id.to_string(),
                "result_summary": delegation.result_summary.clone(),
            })),
        },
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, child_run.thread_id).await?;
    let _ = refresh_thread_status_on(&tx, parent_run.thread_id).await?;
    tx.commit().await?;
    Ok(Some((child_run, delegation, inbox_row, parent_run)))
}

/// Fails a child Run and its delegation, then delivers the failure to and
/// wakes the parent atomically. Repeating the same failure returns the durable
/// terminal records instead of creating another inbox item.
pub async fn fail_child_delegation(
    db: &DatabaseConnection,
    input: AgentChildFailureInput,
) -> Result<
    (
        run::Model,
        delegation::Model,
        inbox_model::Model,
        run::Model,
    ),
    AgentDbError,
> {
    let error_message = input.error_message.trim().to_string();
    if error_message.is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent child error_message 不能为空".into(),
        ));
    }
    let child_identity = run::Entity::find_by_id(input.child_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let delegation_identity = delegation::Entity::find()
        .filter(delegation::Column::ChildRunId.eq(input.child_run_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent child run 缺少 delegation: {}",
                input.child_run_id
            ))
        })?;
    let parent_identity = run::Entity::find_by_id(delegation_identity.parent_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                delegation_identity.parent_run_id
            ))
        })?;
    if child_identity.thread_id != delegation_identity.child_thread_id
        || parent_identity.thread_id != delegation_identity.parent_thread_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation lineage 不一致: {}",
            delegation_identity.id
        )));
    }

    let tx = db.begin().await?;
    lock_threads_on(&tx, [child_identity.thread_id, parent_identity.thread_id]).await?;
    let locked_runs = lock_runs_on(&tx, [child_identity.id, parent_identity.id]).await?;
    let child_run = locked_runs
        .iter()
        .find(|row| row.id == child_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let parent_run = locked_runs
        .iter()
        .find(|row| row.id == parent_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent parent run 不存在: {}", parent_identity.id))
        })?;
    let timestamp = now();
    let mut delegation = lock_delegations_on(&tx, [delegation_identity.id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent delegation 不存在: {}",
                delegation_identity.id
            ))
        })?;
    let parent_execution = if let Some(execution_id) = delegation.parent_execution_id {
        let row = execution::Entity::find_by_id(execution_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!(
                    "Agent parent delegation execution 不存在: {execution_id}"
                ))
            })?;
        if row.run_id != parent_run.id
            || row.execution_kind != "delegation"
            || row.tool_name.as_deref() != Some("delegate")
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent delegation execution 关联不一致: {execution_id}"
            )));
        }
        Some(row)
    } else {
        None
    };

    if child_run.status == "failed" && delegation.status == "failed" {
        let inbox_row = inbox_model::Entity::find()
            .filter(inbox_model::Column::ThreadId.eq(parent_run.thread_id))
            .filter(inbox_model::Column::DedupeKey.eq(format!("child-failed:{}", delegation.id)))
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent child failure inbox 缺失: {}", delegation.id))
            })?;
        tx.commit().await?;
        return Ok((child_run, delegation, inbox_row, parent_run));
    }
    validate_live_run_lease(&child_run, &input.child_lease_token, timestamp)?;
    if delegation.child_run_id != Some(child_run.id)
        || delegation.parent_run_id != parent_run.id
        || delegation.status != "running"
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child failure 关联记录不一致: {}",
            input.child_run_id
        )));
    }
    let child_turn = turn::Entity::find_by_id(child_run.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child turn 不存在: {}", child_run.turn_id))
        })?;
    if child_turn.thread_id != child_run.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent child turn 与 run 不一致: {}",
            child_run.id
        )));
    }

    let mut turn_active: turn::ActiveModel = child_turn.into();
    turn_active.status = Set("failed".to_string());
    turn_active.error_message = Set(Some(error_message.clone()));
    turn_active.updated_at = Set(timestamp);
    turn_active.completed_at = Set(Some(timestamp));
    turn_active.update(&tx).await?;

    let mut child_active: run::ActiveModel = child_run.into();
    child_active.status = Set("failed".to_string());
    child_active.waiting_reason = Set(None);
    child_active.is_active = Set(false);
    child_active.last_error = Set(Some(error_message.clone()));
    child_active.worker_id = Set(None);
    child_active.lease_token = Set(None);
    child_active.lease_expires_at = Set(None);
    child_active.updated_at = Set(timestamp);
    child_active.completed_at = Set(Some(timestamp));
    let child_run = child_active.update(&tx).await?;
    terminate_executions_on(
        &tx,
        [child_run.id],
        ExecutionTermination::Failed,
        Some(error_message.as_str()),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(&tx, [child_run.id], Some(error_message.as_str())).await?;

    let mut delegation_active: delegation::ActiveModel = delegation.into();
    delegation_active.status = Set("failed".to_string());
    delegation_active.error_message = Set(Some(error_message.clone()));
    delegation_active.updated_at = Set(timestamp);
    delegation_active.completed_at = Set(Some(timestamp));
    delegation = delegation_active.update(&tx).await?;

    let parent_tool_message = if let Some(parent_execution) = parent_execution.as_ref() {
        let result = json!({
            "delegation_id": delegation.id.to_string(),
            "child_run_id": child_run.id.to_string(),
            "error": error_message.clone(),
        });
        let message = append_parent_delegation_tool_message_on(
            &tx,
            &parent_run,
            &child_run,
            &delegation,
            parent_execution,
            "failed",
            result.clone(),
        )
        .await?;
        finish_parent_delegation_execution_on(
            &tx,
            parent_execution.clone(),
            "failed",
            result,
            "Child Agent 执行失败".into(),
            None,
            Some(error_message.clone()),
            timestamp,
        )
        .await?;
        Some(message)
    } else {
        None
    };

    let mut inbox_payload = json!({
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "error": error_message,
    });
    if let Some(message) = parent_tool_message.as_ref() {
        inbox_payload["message_id"] = json!(message.id.to_string());
    }
    let inbox_row = enqueue_inbox(
        &tx,
        AgentInboxInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            source_kind: "subagent".into(),
            source_thread_id: Some(child_run.thread_id),
            inbox_kind: "child_result".into(),
            delivery_mode: "followup".into(),
            priority: 20,
            dedupe_key: format!("child-failed:{}", delegation.id),
            content: format!(
                "Child Agent failed: {}",
                inbox_payload["error"].as_str().unwrap_or("unknown error")
            ),
            payload_json: Some(inbox_payload),
            available_at: None,
        },
    )
    .await?;
    let parent_run = if parent_run.is_active
        && parent_run.status == "waiting"
        && parent_run.waiting_reason.as_deref() == Some("child")
    {
        let mut parent_active: run::ActiveModel = parent_run.into();
        parent_active.status = Set("queued".to_string());
        parent_active.waiting_reason = Set(None);
        parent_active.worker_id = Set(None);
        parent_active.lease_token = Set(None);
        parent_active.lease_expires_at = Set(None);
        parent_active.next_attempt_at = Set(timestamp);
        parent_active.updated_at = Set(timestamp);
        parent_active.update(&tx).await?
    } else {
        parent_run
    };
    append_run_event_on(
        &tx,
        &child_run,
        "run_failed",
        "failed",
        "Child Agent 执行失败",
        None,
        Some(delegation.id),
        None,
        Some(json!({ "error": delegation.error_message.clone() })),
    )
    .await?;
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            execution_id: delegation.parent_execution_id,
            delegation_id: Some(delegation.id),
            message_id: parent_tool_message.as_ref().map(|message| message.id),
            actor_kind: "agent".into(),
            actor_id: Some(child_run.id.to_string()),
            kind: "child_failed".into(),
            status: "failed".into(),
            public_text: "Child Agent 执行失败，已通知主 Agent".into(),
            payload_json: Some(json!({
                "delegation_id": delegation.id.to_string(),
                "child_run_id": child_run.id.to_string(),
                "error": delegation.error_message.clone(),
            })),
        },
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, child_run.thread_id).await?;
    let _ = refresh_thread_status_on(&tx, parent_run.thread_id).await?;
    tx.commit().await?;
    Ok((child_run, delegation, inbox_row, parent_run))
}
