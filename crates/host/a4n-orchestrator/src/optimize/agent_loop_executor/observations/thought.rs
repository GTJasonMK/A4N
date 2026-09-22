use super::super::{
    AgentLoopThinkObservationInput, AppError, EVENT_KIND_ASSISTANT_UPDATE, EVENT_STATUS_COMPLETED,
    OrchestratorContext, Value, json,
};

impl OrchestratorContext {
    pub(in crate::optimize) async fn record_agent_loop_think_observation(
        &self,
        input: AgentLoopThinkObservationInput<'_>,
    ) -> Result<Value, AppError> {
        let AgentLoopThinkObservationInput {
            turn_id,
            session_id,
            step,
            decision,
            observation,
            think_event_prefix,
        } = input;
        self.append_turn_event(
            turn_id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            decision.public_update_for_non_final()?,
            Some(json!({
                "step": step,
                "action": decision.action.as_str(),
                "arguments": decision.arguments.clone(),
            })),
        )
        .await?;

        self.save_agent_event(
            session_id,
            &format!(
                "{think_event_prefix}第 {step} 步：{}。",
                observation.event_summary
            ),
        )
        .await?;
        Ok(observation.value)
    }
}
