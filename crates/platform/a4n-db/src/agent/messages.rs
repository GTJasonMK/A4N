use super::*;
use sea_orm::ConnectionTrait;

pub(crate) async fn append_message_on<C>(
    db: &C,
    input: AgentMessageInput,
) -> Result<message::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_message_status(&input.status)?;
    // OpenAI-compatible tool-call turns commonly carry an empty assistant
    // content; the durable payload is the tool_calls array in that case.
    let tool_call_decision = input.role == "assistant"
        && input
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("tool_calls"))
            .and_then(Value::as_array)
            .is_some_and(|calls| !calls.is_empty());
    if input.content.trim().is_empty()
        && !tool_call_decision
        && !(input.role == "assistant" && input.status == "streaming")
    {
        return Err(AgentDbError::InvalidArgument("Agent 消息不能为空".into()));
    }
    let digest = sha256_hex(&input.content);
    if let Some(client_id) = input.client_message_id.as_deref()
        && let Some(existing) = message::Entity::find()
            .filter(message::Column::ThreadId.eq(input.thread_id))
            .filter(message::Column::ClientMessageId.eq(client_id))
            .one(db)
            .await?
    {
        if existing.content_sha256 != digest || existing.role != input.role {
            return Err(AgentDbError::Conflict(format!(
                "client_message_id 已用于不同消息: {}",
                client_id
            )));
        }
        return Ok(existing);
    }
    let timestamp = now();
    let active = message::ActiveModel {
        thread_id: Set(input.thread_id),
        turn_id: Set(input.turn_id),
        run_id: Set(input.run_id),
        role: Set(input.role),
        sender_kind: Set(input.sender_kind),
        sender_thread_id: Set(input.sender_thread_id),
        client_message_id: Set(input.client_message_id.clone()),
        content_sha256: Set(digest.clone()),
        content: Set(input.content),
        status: Set(input.status),
        payload_json: Set(input.payload_json),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        ..Default::default()
    };
    let Some(client_id) = input.client_message_id else {
        return active.insert(db).await.map_err(AgentDbError::from);
    };
    message::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([message::Column::ThreadId, message::Column::ClientMessageId])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let existing = message::Entity::find()
        .filter(message::Column::ThreadId.eq(input.thread_id))
        .filter(message::Column::ClientMessageId.eq(&client_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "消息插入或幂等冲突后无法重读: thread={}, client_message_id={client_id}",
                input.thread_id
            ))
        })?;
    if existing.content_sha256 != digest {
        return Err(AgentDbError::Conflict(format!(
            "client_message_id 已用于不同消息: {client_id}"
        )));
    }
    Ok(existing)
}

pub(crate) fn delegation_tool_call_id(
    execution: &execution::Model,
) -> Result<String, AgentDbError> {
    execution
        .execution_key
        .strip_prefix("tool-call:")
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            AgentDbError::Conflict(format!(
                "Agent delegation execution 缺少可恢复的 tool_call_id: {}",
                execution.id
            ))
        })
}

pub(crate) async fn append_parent_delegation_tool_message_on<C>(
    db: &C,
    parent_run: &run::Model,
    child_run: &run::Model,
    delegation: &delegation::Model,
    parent_execution: &execution::Model,
    status: &str,
    result: Value,
) -> Result<message::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if parent_execution.run_id != parent_run.id
        || parent_execution.execution_kind != "delegation"
        || parent_execution.tool_name.as_deref() != Some("delegate")
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent delegation execution 关联不一致: {}",
            parent_execution.id
        )));
    }
    let tool_call_id = delegation_tool_call_id(parent_execution)?;
    let payload = json!({
        "execution_id": parent_execution.id.to_string(),
        "tool": "delegate",
        "tool_call_id": tool_call_id,
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "status": status,
        "result": result,
    });
    // Keep the durable tool message content identical to the execution's
    // `result_json`.  A resumed parent may replay the succeeded execution via
    // the normal tool-observation path; matching content makes that replay
    // idempotent instead of colliding on `agent-tool:{execution_id}`.
    let content = serde_json::to_string(&result).map_err(|error| {
        AgentDbError::InvalidArgument(format!("无法序列化 delegation result: {error}"))
    })?;
    append_message_on(
        db,
        AgentMessageInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            role: "tool".into(),
            sender_kind: "tool".into(),
            sender_thread_id: Some(child_run.thread_id),
            client_message_id: Some(format!("agent-tool:{}", parent_execution.id)),
            content,
            status: "complete".into(),
            payload_json: Some(payload),
        },
    )
    .await
}

pub(crate) async fn finish_parent_delegation_execution_on<C>(
    db: &C,
    parent_execution: execution::Model,
    status: &str,
    result_json: Value,
    result_summary: String,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
    timestamp: NaiveDateTime,
) -> Result<execution::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if is_execution_terminal(&parent_execution.status) {
        if parent_execution.status == status {
            return Ok(parent_execution);
        }
        return Err(AgentDbError::Conflict(format!(
            "Agent parent delegation execution 已终止，不能从 {} 变为 {}: {}",
            parent_execution.status, status, parent_execution.id
        )));
    }
    if !matches!(
        parent_execution.status.as_str(),
        "prepared" | "awaiting_user" | "dispatching" | "running"
    ) {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent delegation execution 不在可完成状态: {} ({})",
            parent_execution.id, parent_execution.status
        )));
    }
    let mut active: execution::ActiveModel = parent_execution.into();
    active.status = Set(status.to_string());
    active.result_json = Set(Some(result_json));
    active.result_summary = Set(Some(result_summary));
    active.result_refs_json = Set(result_refs_json);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.update(db).await.map_err(AgentDbError::from)
}

/// Projects an expired child lease to its parent. The child run is already
/// transitioned to `recovery_required` by the caller; this helper closes the
/// delegation/execution, appends one durable tool observation, enqueues one
/// child-result inbox item and wakes a parent waiting on that child.
pub(crate) async fn recover_child_delegation_on<C>(
    db: &C,
    child_run: &run::Model,
    parent_run: &run::Model,
    error_message: Option<&str>,
    timestamp: NaiveDateTime,
) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let Some(mut delegation) = delegation::Entity::find()
        .filter(delegation::Column::ChildRunId.eq(child_run.id))
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Err(AgentDbError::Conflict(format!(
            "Agent child run 缺少 delegation，无法通知 parent: {}",
            child_run.id
        )));
    };
    if delegation.parent_run_id != parent_run.id
        || delegation.parent_thread_id != parent_run.thread_id
        || delegation.child_thread_id != child_run.thread_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child recovery delegation lineage 不一致: {}",
            delegation.id
        )));
    }
    if is_delegation_terminal(&delegation.status) {
        return Ok(());
    }
    let reason = error_message
        .unwrap_or("Agent child lease expired; explicit reconciliation required")
        .to_string();
    let mut delegation_active: delegation::ActiveModel = delegation.clone().into();
    delegation_active.status = Set("interrupted".into());
    delegation_active.error_message = Set(Some(reason.clone()));
    delegation_active.result_summary = Set(Some("Child Agent 进入 recovery_required".into()));
    delegation_active.updated_at = Set(timestamp);
    delegation_active.completed_at = Set(Some(timestamp));
    delegation = delegation_active.update(db).await?;

    append_run_event_on(
        db,
        child_run,
        "run_recovery_required",
        "recovery_required",
        "Child Agent 需要显式恢复或对账",
        None,
        Some(delegation.id),
        None,
        Some(json!({ "reason": reason.clone() })),
    )
    .await?;

    let mut inbox_payload = json!({
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "recovery_required": true,
        "reason": reason.clone(),
    });
    if let Some(execution_id) = delegation.parent_execution_id {
        let parent_execution = execution::Entity::find_by_id(execution_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!(
                    "Agent parent delegation execution 不存在: {execution_id}"
                ))
            })?;
        let result = json!({
            "delegation_id": delegation.id.to_string(),
            "child_run_id": child_run.id.to_string(),
            "recovery_required": true,
            "reason": reason.clone(),
        });
        let message = append_parent_delegation_tool_message_on(
            db,
            parent_run,
            child_run,
            &delegation,
            &parent_execution,
            "outcome_unknown",
            result.clone(),
        )
        .await?;
        finish_parent_delegation_execution_on(
            db,
            parent_execution,
            "outcome_unknown",
            result,
            "Child Agent 需要显式恢复或对账".into(),
            None,
            Some(reason.clone()),
            timestamp,
        )
        .await?;
        inbox_payload["message_id"] = json!(message.id.to_string());
    }
    enqueue_inbox(
        db,
        AgentInboxInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            source_kind: "subagent".into(),
            source_thread_id: Some(child_run.thread_id),
            inbox_kind: "child_result".into(),
            delivery_mode: "followup".into(),
            priority: 20,
            dedupe_key: format!("child-recovery:{}:{}", delegation.id, child_run.id),
            content: format!("Child Agent recovery required: {reason}"),
            payload_json: Some(inbox_payload),
            available_at: None,
        },
    )
    .await?;
    append_event_on(
        db,
        AgentEventInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            execution_id: delegation.parent_execution_id,
            delegation_id: Some(delegation.id),
            message_id: None,
            actor_kind: "agent".into(),
            actor_id: Some(child_run.id.to_string()),
            kind: "child_recovery_required".into(),
            status: "interrupted".into(),
            public_text: "Child Agent 需要显式恢复或对账".into(),
            payload_json: Some(json!({ "reason": reason })),
        },
    )
    .await?;
    if parent_run.is_active
        && parent_run.status == "waiting"
        && parent_run.waiting_reason.as_deref() == Some("child")
    {
        let mut parent_active: run::ActiveModel = parent_run.clone().into();
        parent_active.status = Set("queued".into());
        parent_active.waiting_reason = Set(None);
        parent_active.worker_id = Set(None);
        parent_active.lease_token = Set(None);
        parent_active.lease_expires_at = Set(None);
        parent_active.next_attempt_at = Set(timestamp);
        parent_active.updated_at = Set(timestamp);
        parent_active.update(db).await?;
    }
    Ok(())
}

pub async fn append_message<C>(
    db: &C,
    input: AgentMessageInput,
) -> Result<AgentMessageRecord, AgentDbError>
where
    C: ConnectionTrait,
{
    append_message_on(db, input).await.map(Into::into)
}

/// Appends a message only while the caller still owns the Run lease.
///
/// Tool output is part of the durable execution trace; fencing it prevents a
/// stale worker from adding observations after recovery has transferred the
/// run to another owner.
pub async fn append_message_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    input: AgentMessageInput,
) -> Result<AgentMessageRecord, AgentDbError> {
    let tx = db.begin().await?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    if input.run_id != Some(run_id) || input.thread_id != run_row.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent message 与 run 不一致: {run_id}"
        )));
    }
    let row = append_message_on(&tx, input).await?;
    append_message_event_on(&tx, &row, "message_appended", "complete").await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Stores a context summary only while the worker still owns the Run lease.
/// The summary is durable thread state, so a recovered worker cannot overwrite
/// a newer summary from the next lease holder.
pub async fn update_context_summary_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    context_summary: Value,
) -> Result<AgentThreadRecord, AgentDbError> {
    if !context_summary.is_object() {
        return Err(AgentDbError::InvalidArgument(
            "Agent context_summary 必须是 JSON 对象".into(),
        ));
    }
    let serialized = serde_json::to_string(&context_summary).map_err(|error| {
        AgentDbError::InvalidArgument(format!("Agent context_summary 无效: {error}"))
    })?;
    if serialized.len() > 256 * 1024 {
        return Err(AgentDbError::InvalidArgument(
            "Agent context_summary 超过 256 KiB 限制".into(),
        ));
    }
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    let thread_row = thread::Entity::find_by_id(identity.thread_id)
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
    validate_live_run_lease(&run_row, lease_token, now())?;
    if run_row.thread_id != thread_row.id {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 与 thread 不一致: {run_id}"
        )));
    }
    let covered = context_summary
        .get("covered_through_sequence")
        .and_then(Value::as_i64)
        .filter(|id| *id > 0)
        .ok_or_else(|| AgentDbError::InvalidArgument("摘要缺少有效消息边界".into()))?;
    let boundary = message::Entity::find_by_id(covered)
        .one(&tx)
        .await?
        .filter(|message| message.thread_id == thread_row.id);
    if boundary.is_none() {
        return Err(AgentDbError::Conflict(
            "摘要消息边界不属于当前 Thread".into(),
        ));
    }
    let previous = thread_row
        .context_summary
        .as_deref()
        .map(serde_json::from_str::<Value>)
        .transpose()
        .map_err(|error| AgentDbError::InvalidArgument(error.to_string()))?;
    if let Some(previous) = &previous
        && previous
            .get("covered_through_sequence")
            .and_then(Value::as_i64)
            .is_some_and(|id| id >= covered)
    {
        if previous == &context_summary {
            tx.commit().await?;
            return Ok(thread_row.into());
        }
        return Err(AgentDbError::Conflict(
            "不能覆盖相同或较新消息边界的摘要".into(),
        ));
    }
    let receipts = execution::Entity::find()
        .filter(execution::Column::RunId.eq(run_id))
        .filter(execution::Column::ExecutionKind.eq("model_call"))
        .filter(execution::Column::Status.eq("succeeded"))
        .all(&tx)
        .await?;
    let has_receipt = receipts.iter().any(|execution| {
        execution.execution_key.starts_with("context-summary:")
            && execution
                .resource_preconditions_json
                .as_ref()
                .and_then(|value| value.get("covered_through_sequence"))
                .and_then(Value::as_i64)
                == Some(covered)
            && execution
                .result_json
                .as_ref()
                .and_then(|value| value.pointer("/message/content"))
                .and_then(Value::as_str)
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
                .as_ref()
                == Some(&context_summary)
    });
    if !has_receipt {
        return Err(AgentDbError::Conflict(
            "摘要缺少匹配的已验证模型回执".into(),
        ));
    }
    let mut active: thread::ActiveModel = thread_row.into();
    active.context_summary = Set(Some(serialized));
    active.updated_at = Set(now());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(updated.into())
}

/// Upserts one assistant stream projection while the worker still owns its
/// Run lease. The client id is the stream identity, so retries update the
/// same message instead of appending a second assistant response.
pub async fn upsert_assistant_streaming_message_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    input: AgentMessageInput,
) -> Result<AgentMessageRecord, AgentDbError> {
    if input.role != "assistant" || input.status != "streaming" || input.client_message_id.is_none()
    {
        return Err(AgentDbError::InvalidArgument(
            "assistant streaming message 必须带 client_message_id 且状态为 streaming".into(),
        ));
    }
    let client_message_id = input.client_message_id.clone().expect("checked above");
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
    validate_live_run_lease(&run_row, lease_token, now())?;
    if input.thread_id != run_row.thread_id
        || input.turn_id != Some(run_row.turn_id)
        || input.run_id != Some(run_id)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent streaming message 与 run 不一致: {run_id}"
        )));
    }
    let timestamp = now();
    let digest = sha256_hex(&input.content);
    let row = if let Some(existing) = message::Entity::find()
        .filter(message::Column::ThreadId.eq(run_row.thread_id))
        .filter(message::Column::ClientMessageId.eq(&client_message_id))
        .lock_exclusive()
        .one(&tx)
        .await?
    {
        if existing.run_id != Some(run_id)
            || existing.role != "assistant"
            || existing.status != "streaming"
            || !input.content.starts_with(&existing.content)
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent streaming message 不能回退或跨 run 更新: {client_message_id}"
            )));
        }
        let mut active: message::ActiveModel = existing.into();
        active.content_sha256 = Set(digest);
        active.content = Set(input.content);
        active.payload_json = Set(input.payload_json);
        active.updated_at = Set(timestamp);
        active.update(&tx).await?
    } else {
        message::ActiveModel {
            thread_id: Set(input.thread_id),
            turn_id: Set(input.turn_id),
            run_id: Set(input.run_id),
            role: Set(input.role),
            sender_kind: Set(input.sender_kind),
            sender_thread_id: Set(input.sender_thread_id),
            client_message_id: Set(Some(client_message_id)),
            content_sha256: Set(digest),
            content: Set(input.content),
            status: Set(input.status),
            payload_json: Set(input.payload_json),
            created_at: Set(timestamp),
            updated_at: Set(timestamp),
            ..Default::default()
        }
        .insert(&tx)
        .await?
    };
    append_message_event_on(&tx, &row, "message_delta", "streaming").await?;
    tx.commit().await?;
    Ok(row.into())
}

/// Finalizes a streaming assistant projection when the response also carries
/// tool calls. Returning an existing completed row makes retries idempotent.
pub async fn complete_assistant_streaming_message_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    client_message_id: &str,
    content: String,
    payload_json: Value,
) -> Result<Option<AgentMessageRecord>, AgentDbError> {
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
    validate_live_run_lease(&run_row, lease_token, now())?;
    let Some(row) = message::Entity::find()
        .filter(message::Column::ThreadId.eq(run_row.thread_id))
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::ClientMessageId.eq(client_message_id))
        .lock_exclusive()
        .one(&tx)
        .await?
    else {
        tx.commit().await?;
        return Ok(None);
    };
    if row.role != "assistant" || !matches!(row.status.as_str(), "streaming" | "complete") {
        return Err(AgentDbError::Conflict(format!(
            "Agent streaming message 状态不允许完成: {client_message_id}"
        )));
    }
    if row.status == "complete" {
        tx.commit().await?;
        return Ok(Some(row.into()));
    }
    let timestamp = now();
    let mut active: message::ActiveModel = row.into();
    active.content_sha256 = Set(sha256_hex(&content));
    active.content = Set(content);
    active.status = Set("complete".into());
    active.payload_json = Set(Some(payload_json));
    active.updated_at = Set(timestamp);
    let updated = active.update(&tx).await?;
    append_message_event_on(&tx, &updated, "message_completed", "complete").await?;
    tx.commit().await?;
    Ok(Some(updated.into()))
}

pub async fn interrupt_assistant_streaming_message_for_run_lease(
    db: &impl crate::DbExecutor,
    run_id: i64,
    lease_token: &str,
    client_message_id: &str,
    reason: Option<&str>,
) -> Result<(), AgentDbError> {
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
    validate_live_run_lease(&run_row, lease_token, now())?;
    let Some(row) = message::Entity::find()
        .filter(message::Column::ThreadId.eq(run_row.thread_id))
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::ClientMessageId.eq(client_message_id))
        .filter(message::Column::Status.eq("streaming"))
        .lock_exclusive()
        .one(&tx)
        .await?
    else {
        tx.commit().await?;
        return Ok(());
    };
    let mut payload = row.payload_json.clone().unwrap_or_else(|| json!({}));
    if let Value::Object(object) = &mut payload {
        object.insert(
            "interrupted_reason".into(),
            reason.map_or(Value::Null, |value| Value::String(value.to_string())),
        );
    }
    let mut active: message::ActiveModel = row.into();
    active.status = Set("interrupted".into());
    active.payload_json = Set(Some(payload));
    active.updated_at = Set(now());
    let updated = active.update(&tx).await?;
    append_message_event_on(&tx, &updated, "message_interrupted", "interrupted").await?;
    tx.commit().await?;
    Ok(())
}

pub(crate) fn message_projection(row: &message::Model) -> Value {
    json!({
        "id": row.id.to_string(),
        "thread_id": row.thread_id.to_string(),
        "turn_id": row.turn_id.map(|id| id.to_string()),
        "run_id": row.run_id.map(|id| id.to_string()),
        "role": row.role,
        "status": row.status,
        "content": row.content,
        "source_kind": row.sender_kind,
        "source_id": row.sender_thread_id.map(|id| id.to_string()),
        "metadata": row.payload_json,
        "created_at": row.created_at.to_string(),
        "updated_at": row.updated_at.to_string(),
    })
}

pub(crate) async fn append_message_event_on<C: ConnectionTrait>(
    db: &C,
    row: &message::Model,
    kind: &str,
    status: &str,
) -> Result<event::Model, AgentDbError> {
    append_event_on(
        db,
        AgentEventInput {
            thread_id: row.thread_id,
            turn_id: row.turn_id,
            run_id: row.run_id,
            execution_id: None,
            delegation_id: None,
            message_id: Some(row.id),
            actor_kind: "agent".into(),
            actor_id: row.run_id.map(|id| id.to_string()),
            kind: kind.into(),
            status: status.into(),
            public_text: String::new(),
            payload_json: Some(json!({
                "message_id": row.id.to_string(),
                "message": message_projection(row),
            })),
        },
    )
    .await
}

pub(crate) async fn interrupt_streaming_messages_on<C: ConnectionTrait>(
    db: &C,
    run_ids: impl IntoIterator<Item = i64>,
    reason: Option<&str>,
) -> Result<(), AgentDbError> {
    let run_ids = run_ids.into_iter().collect::<Vec<_>>();
    if run_ids.is_empty() {
        return Ok(());
    }
    let rows = message::Entity::find()
        .filter(message::Column::RunId.is_in(run_ids))
        .filter(message::Column::Role.eq("assistant"))
        .filter(message::Column::Status.eq("streaming"))
        .lock_exclusive()
        .all(db)
        .await?;
    let timestamp = now();
    for row in rows {
        let mut payload = row.payload_json.clone().unwrap_or_else(|| json!({}));
        if let Value::Object(object) = &mut payload {
            object.insert(
                "interrupted_reason".into(),
                reason.map_or(Value::Null, |value| Value::String(value.to_string())),
            );
        }
        let mut active: message::ActiveModel = row.into();
        active.status = Set("interrupted".into());
        active.payload_json = Set(Some(payload));
        active.updated_at = Set(timestamp);
        let updated = active.update(db).await?;
        append_message_event_on(db, &updated, "message_interrupted", "interrupted").await?;
    }
    Ok(())
}
