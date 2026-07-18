use super::super::{
    AgentLoopThinkObservationInput, AppError, EVENT_KIND_ASSISTANT_UPDATE, EVENT_KIND_ERROR,
    EVENT_STATUS_COMPLETED, EVENT_STATUS_FAILED, OrchestratorContext, Value, json,
    thinking_observation,
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
            think_event_prefix,
            error_context,
            release_operation,
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

        let observation = match thinking_observation(step, &decision.thought, &decision.arguments) {
            Ok(observation) => observation,
            Err(err) => {
                self.append_turn_event(
                    turn_id,
                    EVENT_KIND_ERROR,
                    EVENT_STATUS_FAILED,
                    &format!("思考参数无效：{err}"),
                    Some(json!({
                        "step": step,
                        "action": decision.action.as_str(),
                        "arguments": decision.arguments.clone(),
                    })),
                )
                .await?;
                self.save_agent_event(
                    session_id,
                    &format!("{error_context} think 参数无效：{err}"),
                )
                .await?;
                if let Some(operation) = release_operation {
                    self.release_in_progress_session_after_failure(session_id, operation, &err)
                        .await?;
                }
                return Err(err);
            }
        };

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
