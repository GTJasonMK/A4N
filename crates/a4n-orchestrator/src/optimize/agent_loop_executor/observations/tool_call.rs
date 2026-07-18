use super::super::{
    AgentLoopDecision, AppError, EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_ERROR,
    EVENT_KIND_TOOL_CALL, EVENT_STATUS_COMPLETED, EVENT_STATUS_FAILED, EVENT_STATUS_RUNNING,
    OrchestratorContext, Value, json, run_tool_arguments,
};

impl OrchestratorContext {
    pub(in crate::optimize) async fn prepare_agent_loop_tool_call(
        &self,
        turn_id: i64,
        session_id: i64,
        step: usize,
        decision: &AgentLoopDecision,
        error_context: &str,
        release_operation: Option<&str>,
    ) -> Result<(String, Value), AppError> {
        let (tool_name, tool_arguments) = match run_tool_arguments(&decision.arguments) {
            Ok(value) => value,
            Err(err) => {
                self.append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("工具调用参数无效：{err}"),
                    Some(json!({
                        "step": step,
                        "action": decision.action.as_str(),
                        "arguments": decision.arguments.clone(),
                    })),
                )
                .await?;
                self.save_agent_event(
                    session_id,
                    &format!("{error_context} run_tool 参数无效：{err}"),
                )
                .await?;
                if let Some(operation) = release_operation {
                    self.release_in_progress_session_after_failure(session_id, operation, &err)
                        .await?;
                }
                return Err(err);
            }
        };
        let tool_name = tool_name.to_string();
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            decision.public_update_for_non_final()?,
            Some(json!({
                "step": step,
                "action": decision.action.as_str(),
                "tool": tool_name,
                "arguments": tool_arguments.clone(),
            })),
        )
        .await?;
        self.append_turn_event(
            turn_id,
            EVENT_KIND_TOOL_CALL,
            EVENT_STATUS_RUNNING,
            &format!("调用工具：{tool_name}"),
            Some(json!({
                "step": step,
                "tool": tool_name,
                "arguments": tool_arguments.clone(),
            })),
        )
        .await?;
        Ok((tool_name, tool_arguments))
    }
}
