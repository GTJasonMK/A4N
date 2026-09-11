mod contracts;
mod persistence;

use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use super::errors::workflow_checkpoint_contract_app_error;

pub use contracts::{
    CompletedWorkflowCheckpointContractError, CompletedWorkflowCheckpointInput,
    required_checkpoint_field, required_checkpoint_json, workflow_checkpoint_fingerprint_sha256,
};

pub(crate) fn attach_llm_call_checkpoint_trace(
    request: &mut ChatRequest,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<(), AppError> {
    let input_fingerprint_sha256 =
        a4n_db::workflow::llm_call_checkpoint_fingerprint_sha256(input_fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    if let Some(trace_context) = request.trace_context.as_mut() {
        trace_context.workflow_run_id = Some(workflow_run_id);
        trace_context.call_key = Some(call_key.to_string());
        trace_context.input_fingerprint_sha256 = Some(input_fingerprint_sha256);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
