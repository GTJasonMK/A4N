use super::super::{
    AgentLoopDecision, AgentLoopDecisionRequest, AgentLoopExecutionError, OrchestratorContext,
};

mod call;
mod contracts;
mod persistence;
mod preparation;
mod repair;
mod rules;

use call::execute_agent_loop_decision_call;
use contracts::PreparedAgentDecisionCall;
use persistence::persist_agent_loop_decision_call;
use preparation::prepare_agent_loop_decision_call;

impl OrchestratorContext {
    pub(in crate::optimize) async fn request_agent_loop_decision(
        &self,
        turn_id: i64,
        session_id: i64,
        request: AgentLoopDecisionRequest<'_>,
    ) -> Result<AgentLoopDecision, AgentLoopExecutionError> {
        let PreparedAgentDecisionCall {
            chat_request,
            call_key,
            input_fingerprint_json,
        } = prepare_agent_loop_decision_call(self, session_id, &request).await?;
        let result = execute_agent_loop_decision_call(
            self,
            turn_id,
            session_id,
            &request,
            chat_request,
            &call_key,
            &input_fingerprint_json,
        )
        .await?;
        persist_agent_loop_decision_call(
            self,
            session_id,
            &request,
            call_key,
            input_fingerprint_json,
            result,
        )
        .await
        .map_err(AgentLoopExecutionError::App)
    }
}
