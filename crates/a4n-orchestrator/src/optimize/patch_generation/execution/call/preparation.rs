use a4n_db::workflow;
use a4n_error::AppError;
use a4n_llm::{provider::ChatRequest, trace::context_aware_chat_request_trace_json};

use crate::{
    optimize::{OrchestratorContext, optimize_llm_call_input_fingerprint_json},
    workflow::workflow_checkpoint_contract_app_error,
};

use super::contracts::PreparedPatchGenerationCall;

pub(super) async fn prepare_patch_generation_call(
    service: &OrchestratorContext,
    session_id: i64,
    workflow_run_id: i64,
    mut request: ChatRequest,
) -> Result<PreparedPatchGenerationCall, AppError> {
    let call_key = format!("optimization_session:{session_id}:patch");
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = None;
    }
    let input_fingerprint_json = optimize_llm_call_input_fingerprint_json(&request)?;
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let llm_request_payload =
        serde_json::to_string_pretty(&context_aware_chat_request_trace_json(&request)?)?;
    service
        .record_llm_request(session_id, "局部补丁生成", &llm_request_payload)
        .await?;
    service
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;
    Ok(PreparedPatchGenerationCall {
        request,
        call_key,
        input_fingerprint_json,
    })
}
