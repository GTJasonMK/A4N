use super::*;
use a4n_db_models::models::agent::inbox as inbox_model;
use a4n_db_models::models::generation::{generation_task_checkpoint, workflow_run};
use a4n_domain::agent::AgentCompletionBlocker;
use sea_orm::ConnectionTrait;

/// Commit the tool receipt, visible question and wait together. A fast user
/// reply already in the inbox requeues the Run instead of leaving it asleep.
pub async fn request_user_input(
    db: &impl crate::DbExecutor,
    input: AgentQuestionInput,
) -> Result<AgentMessageRecord, AgentDbError> {
    let identity = run_by_id(db, input.run_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let tx = db.begin().await?;
    lock_threads_on(&tx, [identity.thread_id]).await?;
    let owner = lock_runs_on(&tx, [input.run_id]).await?.remove(0);
    let execution = execution::Entity::find_by_id(input.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .filter(|execution| {
            execution.run_id == owner.id
                && execution.tool_name.as_deref() == Some("request_agent_input")
        })
        .ok_or_else(|| AgentDbError::Conflict("提问不属于当前 Run 的工具执行".into()))?;
    if execution
        .arguments_json
        .as_ref()
        .and_then(|value| value.get("question"))
        .and_then(Value::as_str)
        != Some(input.question.as_str())
    {
        return Err(AgentDbError::Conflict(
            "提问内容与已保存的工具参数不一致".into(),
        ));
    }
    let question_key = format!("agent-notice:{}", execution.id);
    if execution.status == "succeeded" {
        return message::Entity::find()
            .filter(message::Column::RunId.eq(owner.id))
            .filter(message::Column::ClientMessageId.eq(question_key))
            .one(&tx)
            .await?
            .map(AgentMessageRecord::from)
            .ok_or_else(|| AgentDbError::Conflict("提问执行完成但缺少消息".into()));
    }
    validate_live_run_lease(&owner, &input.lease_token, now())?;
    if !matches!(execution.status.as_str(), "prepared" | "running") {
        return Err(AgentDbError::Conflict("当前提问执行不可派发".into()));
    }
    let call_id = execution
        .execution_key
        .strip_prefix("tool-call:")
        .ok_or_else(|| AgentDbError::InvalidArgument("提问缺少 tool call 身份".into()))?
        .to_string();
    let timestamp = now();
    let mut active: execution::ActiveModel = execution.into();
    active.status = Set("succeeded".into());
    active.result_json = Set(Some(
        json!({"question": input.question, "waiting_for_user": true}),
    ));
    active.result_summary = Set(Some("等待用户回复".into()));
    active.completed_at = Set(Some(timestamp));
    active.updated_at = Set(timestamp);
    active.update(&tx).await?;
    let tool_message = append_message_on(&tx, AgentMessageInput {
        thread_id: owner.thread_id, turn_id: Some(owner.turn_id), run_id: Some(owner.id), role: "tool".into(), sender_kind: "tool".into(), sender_thread_id: None,
        client_message_id: Some(format!("agent-tool:{}", input.execution_id)), content: serde_json::to_string(&input.observation).map_err(|error| AgentDbError::InvalidArgument(error.to_string()))?, status: "complete".into(),
        payload_json: Some(json!({"execution_id": input.execution_id.to_string(), "tool": "request_agent_input", "tool_call_id": call_id, "result": input.observation})),
    }).await?;
    append_message_event_on(&tx, &tool_message, "message_appended", "complete").await?;
    let question = append_message_on(
        &tx,
        AgentMessageInput {
            thread_id: owner.thread_id,
            turn_id: Some(owner.turn_id),
            run_id: Some(owner.id),
            role: "assistant".into(),
            sender_kind: "agent".into(),
            sender_thread_id: None,
            client_message_id: Some(question_key),
            content: input.question,
            status: "complete".into(),
            payload_json: Some(
                json!({"notice_execution_id": input.execution_id, "question": true}),
            ),
        },
    )
    .await?;
    append_message_event_on(&tx, &question, "message_appended", "complete").await?;
    let answered = inbox_model::Entity::find()
        .filter(inbox_model::Column::RunId.eq(owner.id))
        .filter(inbox_model::Column::SourceKind.eq("user"))
        .filter(inbox_model::Column::Status.is_in(["pending", "claimed"]))
        .one(&tx)
        .await?
        .is_some();
    update_run_state_on(
        &tx,
        owner.id,
        Some(&input.lease_token),
        if answered { "queued" } else { "waiting" },
        (!answered).then_some("user"),
        None,
    )
    .await?;
    refresh_thread_status_on(&tx, owner.thread_id).await?;
    tx.commit().await?;
    Ok(question.into())
}

/// Called inside the finalizer's Thread/Run transaction. Only current durable
/// obligations block completion; historical failed attempts are not a gate.
pub async fn completion_blockers<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
) -> Result<Vec<AgentCompletionBlocker>, AgentDbError> {
    let mut blockers = Vec::new();
    for child in delegation::Entity::find()
        .filter(delegation::Column::ParentRunId.eq(run_id))
        .all(db)
        .await?
    {
        if !matches!(child.status.as_str(), "accepted" | "cancelled") {
            blockers.push(AgentCompletionBlocker {
                kind: "delegation".into(),
                id: child.id.to_string(),
                status: child.status,
            });
        }
    }
    let executions = execution::Entity::find()
        .filter(execution::Column::RunId.eq(run_id))
        .all(db)
        .await?;
    let generation_ids = executions
        .iter()
        .filter_map(|execution| {
            execution
                .resource_preconditions_json
                .as_ref()?
                .get("generation_task_id")?
                .as_str()
        })
        .collect::<Vec<_>>();
    let generations = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::TaskId.is_in(generation_ids))
        .all(db)
        .await?;
    let workflow_ids = executions
        .iter()
        .filter_map(|execution| {
            execution
                .resource_preconditions_json
                .as_ref()?
                .get("workflow_run_id")?
                .as_i64()
        })
        .collect::<Vec<_>>();
    let workflows = workflow_run::Entity::find()
        .filter(workflow_run::Column::Id.is_in(workflow_ids))
        .all(db)
        .await?;
    let mut seen = HashSet::new();
    for execution in executions {
        if execution.execution_kind != "model_call"
            && matches!(
                execution.status.as_str(),
                "prepared" | "awaiting_user" | "dispatching" | "running" | "outcome_unknown"
            )
        {
            blockers.push(AgentCompletionBlocker {
                kind: "execution".into(),
                id: execution.id.to_string(),
                status: execution.status.clone(),
            });
        }
        let Some(preconditions) = execution.resource_preconditions_json.as_ref() else {
            continue;
        };
        if let Some(task_id) = preconditions.get("generation_task_id") {
            let task_id = task_id.as_str().ok_or_else(|| {
                AgentDbError::InvalidArgument("Generation owner task_id 无效".into())
            })?;
            let attempt = preconditions
                .get("generation_attempt")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    AgentDbError::InvalidArgument("Generation owner attempt 无效".into())
                })?;
            let id = format!("{task_id}:{attempt}");
            if seen.insert(("generation_task", id.clone())) {
                let Some(task) = generations.iter().find(|task| {
                    task.task_id == task_id && u64::try_from(task.attempt).ok() == Some(attempt)
                }) else {
                    continue;
                };
                // A retried business task supersedes its old failed attempt;
                // old accounting associations must not become permanent gates.
                if generations.iter().any(|newer| {
                    newer.business_key == task.business_key && newer.created_at > task.created_at
                }) {
                    continue;
                }
                let status = task.status.clone();
                if !matches!(status.as_str(), "completed" | "cancelled") {
                    blockers.push(AgentCompletionBlocker {
                        kind: "generation_task".into(),
                        id,
                        status,
                    });
                }
            }
        }
        if let Some(id) = preconditions.get("workflow_run_id") {
            let id = id
                .as_i64()
                .ok_or_else(|| AgentDbError::InvalidArgument("Workflow owner id 无效".into()))?;
            if seen.insert(("workflow_run", id.to_string())) {
                let Some(workflow) = workflows.iter().find(|workflow| workflow.id == id) else {
                    continue;
                };
                if workflows
                    .iter()
                    .any(|newer| newer.business_key == workflow.business_key && newer.id > id)
                {
                    continue;
                }
                let status = workflow.status.clone();
                if !matches!(status.as_str(), "completed" | "cancelled") {
                    blockers.push(AgentCompletionBlocker {
                        kind: "workflow_run".into(),
                        id: id.to_string(),
                        status,
                    });
                }
            }
        }
    }
    Ok(blockers)
}
