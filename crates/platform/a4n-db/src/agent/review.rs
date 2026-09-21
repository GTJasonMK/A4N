use super::*;
use a4n_domain::agent::AgentDelegationReviewInput;

/// Review and evidence commit together; a final message never accepts artifacts.
pub async fn review_delegation(
    db: &DatabaseConnection,
    input: AgentDelegationReviewInput,
) -> Result<delegation::Model, AgentDbError> {
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
    let owner = thread::Entity::find_by_id(identity.parent_thread_id)
        .lock_exclusive()
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
    if row.child_run_id != Some(input.expected_child_run_id) {
        return Err(AgentDbError::Conflict(
            "子任务版本已变化，请重新读取结果".into(),
        ));
    }
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
    if row.status == input.status
        && previous.as_ref().is_some_and(|event| {
            event.payload_json.as_ref().and_then(|p| p.get("review")) == Some(&payload)
        })
    {
        tx.commit().await?;
        return Ok(row);
    }
    if row.status != "awaiting_acceptance" {
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
    let mut active: delegation::ActiveModel = row.into();
    active.status = Set(input.status.clone());
    active.result_summary = Set(Some(input.reason));
    active.updated_at = Set(timestamp);
    active.completed_at = Set((input.status == "accepted").then_some(timestamp));
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
    Ok(reviewed)
}

fn same_resource_version(a: &Value, b: &Value) -> bool {
    ["kind", "id", "revision", "hash"]
        .iter()
        .all(|key| a.get(key).unwrap_or(&Value::Null) == b.get(key).unwrap_or(&Value::Null))
}
