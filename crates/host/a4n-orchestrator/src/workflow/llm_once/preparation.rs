use std::collections::HashMap;

use a4n_db::workflow;
use a4n_error::AppError;
use serde_json::json;

use crate::workflow::workflow_checkpoint_contract_app_error;

use super::{PreparedWorkflowLlmOnce, WorkflowLlmOnceInput};

pub(super) async fn prepare_workflow_llm_once(
    input: &WorkflowLlmOnceInput<'_>,
) -> Result<PreparedWorkflowLlmOnce, AppError> {
    let system_prompt = input
        .prompt_renderer
        .render_prompt_for_call(
            input.prompt_assembly_call_key,
            input.prompt_name,
            &HashMap::new(),
            Some(input.project_id),
        )
        .await?;
    let request = super::workflow_json_request(
        system_prompt.clone(),
        input.user_prompt.clone(),
        input.prompt_name,
    )?;
    let fingerprint_json = json!({
        "request": request.snapshot_value()?,
        "workflow": input.workflow,
        "step_key": input.step_key.as_str(),
        "prompt_name": input.prompt_name,
        "system_prompt": system_prompt,
        "user_prompt": input.user_prompt.as_str(),
        "temperature": super::WORKFLOW_LLM_TEMPERATURE,
    });
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    Ok(PreparedWorkflowLlmOnce {
        system_prompt,
        fingerprint_json,
        input_fingerprint_sha256,
    })
}
