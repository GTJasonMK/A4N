use a4n_core::error::AppError;
use a4n_db::workflow;
use a4n_llm::provider::{ChatRequest, LlmProvider};
use serde_json::Value;

use super::super::checkpoint::chapter_generation_call_fingerprint_json;
use crate::workflow::workflow_checkpoint_contract_app_error;

pub(super) fn prepare_chapter_generation_request(
    project_id: i64,
    chapter_number: i32,
    workflow_run_id: i64,
    mut request: ChatRequest,
    llm: &dyn LlmProvider,
) -> Result<(ChatRequest, ChatRequest, String, Value, String), AppError> {
    let call_key = format!("chapter_generation:chapter:{chapter_number}");
    let input_fingerprint_json =
        chapter_generation_call_fingerprint_json(project_id, chapter_number, &request)?;
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    let request_trace = request.clone();
    let model_name = llm.model_name().to_string();
    Ok((
        request,
        request_trace,
        call_key,
        input_fingerprint_json,
        model_name,
    ))
}
