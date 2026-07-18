use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use super::super::super::{
    AgentLoopDecisionRequest, AppError, EVENT_KIND_ERROR, EVENT_STATUS_FAILED, OptimizeLlmStream,
    OrchestratorContext, collect_streaming_chat_response, json, workflow,
    workflow_checkpoint_contract_app_error, workflow_read_app_error,
};
use super::super::checkpoint::restored_agent_loop_decision;
use super::contracts::AgentDecisionCallResult;
use super::repair::repair_agent_loop_decision;

pub(super) async fn execute_agent_loop_decision_call(
    context: &OrchestratorContext,
    turn_id: i64,
    session_id: i64,
    stream: &mut Option<OptimizeLlmStream<'_>>,
    request: &AgentLoopDecisionRequest<'_>,
    chat_request: ChatRequest,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<AgentDecisionCallResult, AppError> {
    if let Some(checkpoint) = workflow::load_completed_llm_call_checkpoint(
        context.db.connection(),
        request.workflow_run_id,
        call_key,
        input_fingerprint_json,
    )
    .await
    .map_err(workflow_read_app_error)?
    {
        let checkpoint_json = workflow::required_llm_call_checkpoint_json(checkpoint, call_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
        let (response, decision) = restored_agent_loop_decision(checkpoint_json, call_key)?;
        context
            .save_agent_event(
                session_id,
                &format!(
                    "已恢复 {} 第 {} 步 LLM 调用断点。",
                    request.phase, request.step
                ),
            )
            .await?;
        return Ok(AgentDecisionCallResult {
            response,
            decision,
            restored: true,
        });
    }

    let repair_request = chat_request.clone();
    let response =
        match collect_streaming_chat_response(context.llm.as_ref(), chat_request, stream).await {
            Ok(response) => response,
            Err(err) => {
                context
                    .append_turn_event(
                        turn_id,
                        EVENT_KIND_ERROR,
                        EVENT_STATUS_FAILED,
                        &format!("LLM 调用失败：{err}"),
                        Some(json!({
                            "step": request.step,
                            "phase": request.phase,
                        })),
                    )
                    .await?;
                context
                    .save_agent_event(
                        session_id,
                        &format!("{} 调用失败：{err}", request.llm_error_label),
                    )
                    .await?;
                if let Some(operation) = request.release_operation {
                    context
                        .release_in_progress_session_after_failure(session_id, operation, &err)
                        .await?;
                }
                return Err(err);
            }
        };
    let decision = repair_agent_loop_decision(
        context,
        turn_id,
        session_id,
        request,
        repair_request,
        &response.message.content,
        call_key,
    )
    .await?;
    Ok(AgentDecisionCallResult {
        response,
        decision,
        restored: false,
    })
}
