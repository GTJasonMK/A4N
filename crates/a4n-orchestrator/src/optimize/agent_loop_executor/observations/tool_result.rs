use super::super::{
    AgentLoopAction, AppError, EVENT_KIND_TOOL_RESULT, EVENT_STATUS_COMPLETED, EVENT_STATUS_FAILED,
    FailedAgentLoopToolInput, OrchestratorContext, Value, failed_tool_observation, json,
};

impl OrchestratorContext {
    pub(in crate::optimize) async fn record_failed_agent_loop_tool(
        &self,
        input: FailedAgentLoopToolInput<'_>,
    ) -> Result<Value, AppError> {
        let FailedAgentLoopToolInput {
            turn_id,
            session_id,
            step,
            tool_name,
            tool_arguments,
            err,
            tool_event_prefix,
        } = input;
        let observation = failed_tool_observation(
            step,
            &AgentLoopAction::RunTool,
            tool_name,
            tool_arguments,
            err,
        );
        self.append_turn_event(
            turn_id,
            EVENT_KIND_TOOL_RESULT,
            EVENT_STATUS_FAILED,
            &format!("工具 {tool_name} 执行失败：{err}"),
            Some(observation.clone()),
        )
        .await?;
        self.save_message_with_tool_calls(
            session_id,
            "tool",
            &serde_json::to_string_pretty(&observation)?,
            Some(json!({
                "tool": observation["tool"].clone(),
                "step": step,
            })),
        )
        .await?;
        self.save_agent_event(
            session_id,
            &format!("{tool_event_prefix}工具 {tool_name} 执行失败，已交回 Agent 决策：{err}"),
        )
        .await?;
        Ok(observation)
    }

    pub(in crate::optimize) async fn record_successful_agent_loop_tool(
        &self,
        turn_id: i64,
        session_id: i64,
        step: usize,
        tool_name: &str,
        observation: &Value,
        tool_event_prefix: &str,
    ) -> Result<(), AppError> {
        self.append_turn_event(
            turn_id,
            EVENT_KIND_TOOL_RESULT,
            EVENT_STATUS_COMPLETED,
            &format!("工具 {tool_name} 已返回结果。"),
            Some(observation.clone()),
        )
        .await?;
        self.save_message_with_tool_calls(
            session_id,
            "tool",
            &serde_json::to_string_pretty(observation)?,
            Some(json!({
                "tool": tool_name,
                "step": step,
            })),
        )
        .await?;
        self.save_agent_event(
            session_id,
            &format!("{tool_event_prefix}工具 {tool_name} 已返回观察结果。"),
        )
        .await?;
        Ok(())
    }
}
