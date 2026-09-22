use super::super::{
    AgentLoopDecision, AppError, EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_TOOL_CALL,
    EVENT_STATUS_COMPLETED, EVENT_STATUS_RUNNING, OrchestratorContext, Value, json,
};

impl OrchestratorContext {
    pub(in crate::optimize) async fn record_agent_loop_tool_call(
        &self,
        turn_id: i64,
        step: usize,
        decision: &AgentLoopDecision,
        tool_name: &str,
        tool_arguments: &Value,
    ) -> Result<(), AppError> {
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            decision.public_update_for_non_final()?,
            Some(json!({
                "step": step,
                "action": decision.action.as_str(),
                "tool": tool_name,
                "arguments": tool_arguments,
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
                "arguments": tool_arguments,
            })),
        )
        .await
        .map(drop)
    }
}
