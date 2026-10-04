use super::*;
use a4n_domain::agent::AgentDelegationReviewInput;

/// Review and evidence commit together; a final message never accepts artifacts.
pub async fn review_delegation(
    db: &impl crate::DbExecutor,
    input: AgentDelegationReviewInput,
) -> Result<AgentDelegationRecord, AgentDbError> {
    if !matches!(input.status.as_str(), "accepted" | "revision_requested")
        || input.reason.trim().is_empty()
    {
        return Err(AgentDbError::InvalidArgument(
            "委派审阅必须包含结论和原因".into(),
        ));
    }
    let identity = delegation::Entity::find_by_id(input.delegation_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent delegation 不存在".into()))?;
    let tx = db.begin().await?;
    lock_threads_on(&tx, [identity.parent_thread_id, identity.child_thread_id]).await?;
    let owner = thread::Entity::find_by_id(identity.parent_thread_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent parent thread 不存在".into()))?;
    if owner.owner_user_id != input.user_id {
        return Err(AgentDbError::Conflict("不能审阅其他用户的委派".into()));
    }
    let parent = run::Entity::find_by_id(identity.parent_run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent parent run 不存在".into()))?;
    let row = delegation::Entity::find_by_id(identity.id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent delegation 不存在".into()))?;
    let payload = json!({
        "child_run_id": input.expected_child_run_id.to_string(),
        "reason": input.reason,
        "reviewed_refs": input.reviewed_refs,
    });
    let previous = event::Entity::find()
        .filter(event::Column::DelegationId.eq(row.id))
        .filter(event::Column::Kind.eq("delegation_reviewed"))
        .order_by_desc(event::Column::Sequence)
        .one(&tx)
        .await?;
    if (row.status == input.status || input.status == "revision_requested")
        && previous.as_ref().is_some_and(|event| {
            event.status == input.status
                && event.payload_json.as_ref().and_then(|p| p.get("review")) == Some(&payload)
        })
    {
        tx.commit().await?;
        return Ok(row.into());
    }
    if row.child_run_id != Some(input.expected_child_run_id) {
        return Err(AgentDbError::Conflict(
            "子任务版本已变化，请重新读取结果".into(),
        ));
    }
    if !parent.is_active || !matches!(parent.status.as_str(), "queued" | "running" | "waiting") {
        return Err(AgentDbError::Conflict(
            "父 Agent 当前不可审阅或修订子任务".into(),
        ));
    }
    if row.status != "awaiting_acceptance"
        && !(input.status == "revision_requested"
            && matches!(row.status.as_str(), "failed" | "interrupted"))
    {
        return Err(AgentDbError::Conflict(format!(
            "委派当前不可审阅: {}",
            row.status
        )));
    }
    let delivered = match row.result_refs_json.as_ref() {
        None | Some(Value::Null) => Vec::new(),
        Some(value) => value
            .as_array()
            .cloned()
            .ok_or_else(|| AgentDbError::InvalidArgument("委派结果引用必须是数组".into()))?,
    };
    let mut observed = Vec::new();
    let mut evidence_ids = Vec::new();
    let evidence = execution::Entity::find()
        .filter(execution::Column::RunId.eq(parent.id))
        .filter(execution::Column::Status.eq("succeeded"))
        .filter(execution::Column::Effect.eq("read"))
        .filter(execution::Column::ExecutionKind.is_in(["tool_call", "verification", "wait"]))
        .filter(execution::Column::ToolName.ne("read_agent_execution_result"))
        .all(&tx)
        .await?;
    for evidence in evidence {
        if let Some(refs) = evidence.result_refs_json {
            let refs = refs
                .as_array()
                .ok_or_else(|| AgentDbError::InvalidArgument("审阅证据引用无效".into()))?;
            if refs.iter().any(|seen| {
                input
                    .reviewed_refs
                    .iter()
                    .any(|r| same_resource_version(seen, r))
            }) {
                evidence_ids.push(evidence.id.to_string());
                observed.extend(refs.iter().cloned());
            }
        }
    }
    for resource in &input.reviewed_refs {
        if !observed
            .iter()
            .any(|seen| same_resource_version(seen, resource))
        {
            return Err(AgentDbError::InvalidArgument(
                "审阅引用缺少对应版本的父 Agent 读取证据".into(),
            ));
        }
    }
    if input.status == "accepted"
        && delivered.iter().any(|resource| {
            !input
                .reviewed_refs
                .iter()
                .any(|seen| same_resource_version(seen, resource))
        })
    {
        return Err(AgentDbError::InvalidArgument(
            "接受前必须检查子任务交付的每个资源版本".into(),
        ));
    }
    let timestamp = now();
    let revision = if input.status == "revision_requested" {
        let remaining = run_budget_snapshot(&tx, parent.id).await?.remaining;
        if remaining.max_model_calls == Some(0)
            || remaining.max_steps == Some(0)
            || remaining.max_tokens == Some(0)
            || remaining.max_duration_seconds == Some(0)
        {
            return Err(AgentDbError::BudgetExhausted(parent.id.to_string()));
        }
        let child = run::Entity::find_by_id(input.expected_child_run_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("待修订的 child Run 不存在".into()))?;
        let original = turn::Entity::find_by_id(child.turn_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("待修订的 child Turn 不存在".into()))?;
        let snapshot_required = child
            .usage_json
            .get("runtime_snapshot_required")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                AgentDbError::InvalidArgument("child Run 缺少 snapshot admission".into())
            })?;
        let key = format!("delegation-revision:{}:{}", row.id, child.id);
        let task = format!("{}\n\n本次修订要求：{}", row.objective, input.reason);
        let (turn, child, _) = create_turn_with_run_on(&tx, AgentTurnInput {
            thread_id: row.child_thread_id, client_message_id: key.clone(),
            message: AgentMessageInput {
                thread_id: row.child_thread_id, turn_id: None, run_id: None, role: "user".into(), sender_kind: "agent".into(), sender_thread_id: Some(parent.thread_id),
                client_message_id: Some(key), content: task.clone(), status: "complete".into(), payload_json: Some(json!({"delegation_id": row.id.to_string(), "revises_run_id": input.expected_child_run_id.to_string()})),
            },
            execution_mode: original.execution_mode, model_override_json: original.model_override_json, budget_json: row.budget_json.clone(), runtime_snapshot_required: snapshot_required,
            trigger: "delegation".into(), goal: Some(task),
        }).await?;
        let mut active: run::ActiveModel = child.into();
        active.parent_run_id = Set(Some(parent.id));
        active.root_run_id = Set(Some(parent.root_run_id.unwrap_or(parent.id)));
        let child = active.update(&tx).await?;
        Some((turn.id, child.id))
    } else {
        None
    };
    let mut active: delegation::ActiveModel = row.into();
    active.status = Set(input.status.clone());
    active.result_summary = Set(Some(input.reason));
    active.updated_at = Set(timestamp);
    active.completed_at = Set((input.status == "accepted").then_some(timestamp));
    if let Some((turn_id, run_id)) = revision {
        active.status = Set("running".into());
        active.child_turn_id = Set(Some(turn_id));
        active.child_run_id = Set(Some(run_id));
        // The original delegate call already has a durable result. A revision
        // is delivered through a new inbox item, not by overwriting that call.
        active.parent_execution_id = Set(None);
        active.result_refs_json = Set(None);
        if parent.status != "running" {
            update_run_state_on(&tx, parent.id, None, "waiting", Some("child"), None).await?;
        }
    }
    let reviewed = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &parent,
        "delegation_reviewed",
        &input.status,
        "子任务结果已显式审阅",
        reviewed.parent_execution_id,
        Some(reviewed.id),
        None,
        Some(json!({"review": payload, "evidence_execution_ids": evidence_ids})),
    )
    .await?;
    tx.commit().await?;
    Ok(reviewed.into())
}

fn same_resource_version(a: &Value, b: &Value) -> bool {
    ["kind", "id", "revision", "hash"]
        .iter()
        .all(|key| a.get(key).unwrap_or(&Value::Null) == b.get(key).unwrap_or(&Value::Null))
}
