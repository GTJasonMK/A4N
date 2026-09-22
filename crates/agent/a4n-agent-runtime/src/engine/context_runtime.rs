use super::*;

pub(crate) struct ContextSummaryOutcome {
    pub(crate) applied: bool,
    pub(crate) new_tokens: Option<u64>,
}

pub(crate) async fn compact_context(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    context: &mut AgentRunContext,
    covered_through_sequence: i64,
    source: String,
) -> Result<ContextSummaryOutcome, AppError> {
    let summary = a4n_agent_runtime_contract::deterministic_context_summary(
        context.thread.context_summary.as_ref(),
        context.goal.as_deref(),
        &source,
        covered_through_sequence,
    )?;
    summary.validate()?;
    let summary_value = serde_json::to_value(summary)?;
    state
        .update_agent_context_summary(claimed, summary_value.clone())
        .await?;
    context.thread.context_summary = Some(summary_value);
    Ok(ContextSummaryOutcome {
        applied: true,
        new_tokens: None,
    })
}

pub(crate) fn execution_total_tokens(execution: &AgentExecutionSnapshot) -> Result<u64, AppError> {
    if execution.execution_key.starts_with("model-call:") {
        return model_response(execution)?
            .usage
            .map(|usage| usage.total_tokens as u64)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("Agent model execution {} 缺少 usage", execution.id),
                operation: "恢复 token 预算".into(),
            });
    }
    if execution.execution_key.starts_with("context-summary:") {
        return execution
            .result_json
            .as_ref()
            .and_then(|value| value.pointer("/usage/total_tokens"))
            .and_then(Value::as_u64)
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "Agent context summary execution {} 缺少 usage",
                    execution.id
                ),
                operation: "恢复 token 预算".into(),
            });
    }
    Err(AppError::InvalidState {
        current: format!(
            "未知 Agent model execution key: {}",
            execution.execution_key
        ),
        operation: "恢复模型计数".into(),
    })
}

pub(crate) async fn recover_model_messages(
    message_application: &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort,
    claimed: &AgentWorkerRun,
    context: &mut AgentRunContext,
) -> Result<Option<(String, i64)>, AppError> {
    for execution in &context.executions {
        if !execution.execution_key.starts_with("model-call:") || execution.status != "succeeded" {
            continue;
        }
        let response = model_response(execution)?;
        let protocol = execution
            .arguments_json
            .as_ref()
            .map(agent_protocol_from_request_snapshot)
            .unwrap_or(a4n_domain::agent::AGENT_PROTOCOL_NATIVE_TOOL_CALLING);
        let decision = decode_agent_response(&response, protocol, execution.id)?;
        let (content, calls, is_final) = match decision {
            AgentModelDecision::Final { reply } => (reply, Vec::new(), true),
            AgentModelDecision::Think { content } => (content, Vec::new(), false),
            AgentModelDecision::ToolCalls { content, calls } => (content, calls, false),
        };
        if is_final {
            let input_message_id = execution_input_message_id(execution)?;
            if context.messages.iter().any(|m| {
                m.run_id == Some(claimed.run.id) && m.role == "user" && m.id > input_message_id
            }) {
                continue;
            }
            return Ok(Some((content, input_message_id)));
        }
        let calls = serde_json::to_value(calls)?;
        if !context.messages.iter().any(|message| {
            message.run_id == Some(claimed.run.id)
                && message
                    .payload_json
                    .as_ref()
                    .and_then(|payload| payload.get("tool_calls"))
                    == Some(&calls)
        }) {
            let message = if let Some(client_message_id) =
                stream_client_message_id(claimed.run.id, execution)
            {
                message_application
                    .complete_assistant_streaming_message(
                        claimed,
                        client_message_id,
                        content.clone(),
                        calls.clone(),
                    )
                    .await?
            } else {
                None
            };
            let message = match message {
                Some(message) => message,
                None => {
                    message_application
                        .append_assistant_tool_message(claimed, content, calls)
                        .await?
                }
            };
            context.messages.push(message);
        }
    }
    Ok(None)
}

pub(crate) fn execution_input_message_id(
    execution: &AgentExecutionSnapshot,
) -> Result<i64, AppError> {
    execution
        .resource_preconditions_json
        .as_ref()
        .and_then(|v| v.get("input_message_id"))
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            AppError::InvalidArgument("模型执行缺少已观察的消息边界，不能结束任务".into())
        })
}
