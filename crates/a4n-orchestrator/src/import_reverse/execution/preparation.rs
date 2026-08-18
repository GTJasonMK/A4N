use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use a4n_core::error::AppError;
use a4n_db::workflow;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use crate::OrchestratorContext;
use crate::import_reverse::IMPORT_REVERSE_WORKFLOW_KIND;
use crate::workflow::workflow_checkpoint_contract_app_error;

use super::contracts::PreparedImportReverseCall;

pub(super) async fn prepare_import_reverse_call(
    context: &OrchestratorContext,
    target: ReverseEngineeringTarget,
    mut request: ChatRequest,
    workflow_run_id: Option<i64>,
) -> Result<PreparedImportReverseCall, AppError> {
    let prompt_name = target.prompt_name();
    let call_key = prompt_name.to_string();
    let workflow_run_id = workflow::required_llm_call_workflow_run_id(workflow_run_id, &call_key)
        .map_err(workflow_checkpoint_contract_app_error)?;
    let fingerprint_json = import_reverse_call_fingerprint_json(target, &request)?;
    crate::workflow::attach_llm_call_checkpoint_trace(
        &mut request,
        workflow_run_id,
        &call_key,
        &fingerprint_json,
    )?;
    context
        .mark_workflow_current_step(workflow_run_id, &call_key)
        .await?;

    Ok(PreparedImportReverseCall {
        target,
        request,
        workflow_run_id,
        fingerprint_json,
    })
}

fn import_reverse_call_fingerprint_json(
    target: ReverseEngineeringTarget,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    let prompt_name = target.prompt_name();
    Ok(json!({
        "schema_version": 1,
        "workflow": IMPORT_REVERSE_WORKFLOW_KIND,
        "call_key": prompt_name,
        "prompt_name": prompt_name,
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
    }))
}
