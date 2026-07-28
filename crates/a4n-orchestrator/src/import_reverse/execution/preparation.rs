use a4n_core::error::AppError;
use a4n_db::workflow;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::workflow::{workflow_checkpoint_contract_app_error, workflow_write_app_error};

use super::contracts::PreparedImportReverseCall;

pub(super) async fn prepare_import_reverse_call(
    context: &OrchestratorContext,
    prompt_name: &str,
    mut request: ChatRequest,
    workflow_run_id: Option<i64>,
) -> Result<PreparedImportReverseCall, AppError> {
    let call_key = prompt_name.to_string();
    let workflow_run_id = workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let fingerprint_json = import_reverse_call_fingerprint_json(prompt_name, &request)?;
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.clone());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    workflow::mark_workflow_current_step(context.database.connection(), workflow_run_id, &call_key)
        .await
        .map_err(workflow_write_app_error)?;

    Ok(PreparedImportReverseCall {
        prompt_name: prompt_name.to_string(),
        request,
        workflow_run_id,
        call_key,
        fingerprint_json,
    })
}

fn import_reverse_call_fingerprint_json(
    prompt_name: &str,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "import_reverse_engineer",
        "call_key": prompt_name,
        "prompt_name": prompt_name,
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
    }))
}
