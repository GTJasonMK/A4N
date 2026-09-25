use super::*;

pub(crate) fn next_model_execution_key(
    executions: &[AgentExecutionSnapshot],
) -> Result<String, AppError> {
    let mut highest = 0u64;
    for execution in executions {
        let Some(sequence) = execution.execution_key.strip_prefix("model-call:") else {
            continue;
        };
        if matches!(execution.status.as_str(), "prepared" | "running") {
            return Ok(execution.execution_key.clone());
        }
        highest = highest.max(
            sequence
                .parse::<u64>()
                .map_err(|_| AppError::InvalidState {
                    current: format!("模型执行序号无效：{}", execution.execution_key),
                    operation: "恢复模型执行".into(),
                })?,
        );
    }
    let next = highest
        .checked_add(1)
        .ok_or_else(|| AppError::InvalidState {
            current: "模型执行序号溢出".into(),
            operation: "恢复模型执行".into(),
        })?;
    Ok(format!("model-call:{next}"))
}

/// Providers need only make IDs unique within one response. Persist the
/// validated calls with Run-wide identities while preserving native signatures.
pub(crate) fn namespace_tool_calls(
    response: &mut ChatResponse,
    decision: &mut AgentModelDecision,
    execution_id: i64,
) {
    if response.message.tool_calls.is_none() {
        return;
    }
    if let AgentModelDecision::ToolCalls { calls, .. } = decision {
        for (index, call) in calls.iter_mut().enumerate() {
            call.id = format!("agent-call-{execution_id}-{index}");
        }
        response.message.tool_calls = Some(calls.clone());
    }
}

pub(crate) fn model_response(execution: &AgentExecutionSnapshot) -> Result<ChatResponse, AppError> {
    let mut response: ChatResponse =
        serde_json::from_value(execution.result_json.clone().ok_or_else(|| {
            AppError::InvalidState {
                current: format!(
                    "Agent model execution {} succeeded 但缺少 result_json",
                    execution.id
                ),
                operation: "恢复 Agent 模型响应".into(),
            }
        })?)
        .map_err(AppError::from)?;
    response
        .message
        .preserve_tool_reasoning(response.reasoning_content.as_deref())?;
    Ok(response)
}

pub(crate) fn model_execution_request(
    execution: &AgentExecutionSnapshot,
) -> Result<ChatRequest, AppError> {
    restore_model_request_wire(execution.arguments_json.clone().ok_or_else(|| {
        AppError::InvalidState {
            current: format!(
                "Agent model execution {} 缺少原始 request snapshot",
                execution.id
            ),
            operation: "恢复 Agent model execution request".into(),
        }
    })?)
}

pub(crate) fn stream_client_message_id(
    run_id: i64,
    execution: &AgentExecutionSnapshot,
) -> Option<String> {
    execution
        .execution_key
        .strip_prefix("model-call:")
        .map(|step| format!("agent-stream:{run_id}:{step}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_provider_ids_are_scoped_to_model_execution_and_replay_stably() {
        let mut response = ChatResponse {
            message: ChatMessage::assistant(""),
            finish_reason: Some("tool_calls".into()),
            usage: None,
            reasoning_content: None,
        };
        response.message.tool_calls = Some(vec![ToolCall {
            id: "call_1".into(),
            name: "get_project".into(),
            arguments: json!({"projectId": 7}),
            stream_index: None,
            provider_metadata: Some(
                json!({"gemini": {"id": "native", "thought_signature": "opaque"}}),
            ),
        }]);
        let mut second = response.clone();
        let protocol = a4n_domain::agent::AGENT_PROTOCOL_NATIVE_TOOL_CALLING;
        let mut first_decision = decode_agent_response(&response, protocol, 11).unwrap();
        let mut second_decision = decode_agent_response(&second, protocol, 12).unwrap();
        namespace_tool_calls(&mut response, &mut first_decision, 11);
        namespace_tool_calls(&mut second, &mut second_decision, 12);
        let first = &response.message.tool_calls.as_ref().unwrap()[0];
        let next = &second.message.tool_calls.as_ref().unwrap()[0];
        assert_ne!(first.id, next.id);
        assert_eq!(first.provider_metadata, next.provider_metadata);
        assert_eq!(first.arguments, next.arguments);
        let AgentModelDecision::ToolCalls { calls, .. } =
            decode_agent_response(&response, protocol, 11).unwrap()
        else {
            panic!("expected calls")
        };
        assert_eq!(calls[0].id, first.id);
    }
}
