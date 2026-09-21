use super::*;

pub(crate) fn model_response(execution: &AgentExecutionSnapshot) -> Result<ChatResponse, AppError> {
    serde_json::from_value(
        execution
            .result_json
            .clone()
            .ok_or_else(|| AppError::InvalidState {
                current: format!(
                    "Agent model execution {} succeeded 但缺少 result_json",
                    execution.id
                ),
                operation: "恢复 Agent 模型响应".into(),
            })?,
    )
    .map_err(AppError::from)
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
