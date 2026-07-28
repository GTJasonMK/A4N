use super::super::super::{
    AgentLoopDecisionRequest, AppError, OrchestratorContext, agent_loop_request_payload,
    build_agent_loop_chat_request, optimize_llm_call_input_fingerprint_json, workflow,
    workflow_checkpoint_contract_app_error, workflow_write_app_error,
};

use super::contracts::PreparedAgentDecisionCall;

pub(super) async fn prepare_agent_loop_decision_call(
    context: &OrchestratorContext,
    session_id: i64,
    request: &AgentLoopDecisionRequest<'_>,
) -> Result<PreparedAgentDecisionCall, AppError> {
    let mut chat_request = match build_agent_loop_chat_request(
        request.project_id,
        Some(session_id),
        request.workflow,
        request.system_prompt.to_string(),
        request.base_user_message,
        request.observations,
        request.step,
    ) {
        Ok(request) => request,
        Err(err) => {
            context
                .save_agent_event(
                    session_id,
                    &format!(
                        "构建 {} 第 {} 步输入失败：{err}",
                        request.build_error_label, request.step
                    ),
                )
                .await?;
            return Err(err);
        }
    };
    let call_key = format!(
        "optimization_session:{}:{}:step:{}",
        session_id, request.workflow, request.step
    );
    if let Some(trace_context) = chat_request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(request.workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = None;
    }
    let input_fingerprint_json = optimize_llm_call_input_fingerprint_json(&chat_request)?;
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = chat_request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    context
        .save_agent_event(
            session_id,
            &format!(
                "{} 第 {} 步：请求下一步决策。",
                request.request_event_label, request.step
            ),
        )
        .await?;
    let llm_request_payload = agent_loop_request_payload(&chat_request)?;
    context
        .record_llm_request(session_id, request.phase, &llm_request_payload)
        .await?;
    workflow::mark_workflow_current_step(
        context.database.connection(),
        request.workflow_run_id,
        &call_key,
    )
    .await
    .map_err(workflow_write_app_error)?;
    Ok(PreparedAgentDecisionCall {
        chat_request,
        call_key,
        input_fingerprint_json,
    })
}
