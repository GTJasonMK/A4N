use serde_json::Value;

use crate::workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput};

use super::super::super::{
    AgentLoopDecision, AgentLoopDecisionRequest, AppError, OrchestratorContext,
};
use super::super::checkpoint::agent_loop_decision_json;
use super::contracts::AgentDecisionCallResult;
use super::rules::agent_loop_call_order;

pub(super) async fn persist_agent_loop_decision_call(
    context: &OrchestratorContext,
    session_id: i64,
    request: &AgentLoopDecisionRequest<'_>,
    call_key: String,
    input_fingerprint_json: Value,
    result: AgentDecisionCallResult,
) -> Result<AgentLoopDecision, AppError> {
    let AgentDecisionCallResult {
        response,
        decision,
        restored,
    } = result;
    context
        .record_llm_response(session_id, request.phase, &response.message.content)
        .await?;
    if !restored {
        let usage = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        context
            .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: request.workflow_run_id,
                call_key,
                call_order: agent_loop_call_order(request.step)?,
                input_fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: agent_loop_decision_json(&decision),
                    response_content: response.message.content,
                    usage,
                    finish_reason: response.finish_reason,
                    artifacts_json: None,
                },
            })
            .await?;
    }
    if decision.action != crate::optimize::agent_loop::AgentLoopAction::Final {
        let task = request.generation.session.snapshot();
        context
            .agent()
            .publish_generation_progress(
                &task.task_id,
                task.attempt,
                request.step,
                decision.public_update_for_non_final()?,
            )
            .await?;
    }
    Ok(decision)
}
