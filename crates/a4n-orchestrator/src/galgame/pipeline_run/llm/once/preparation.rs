use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_db::workflow;
use serde_json::json;

use crate::workflow::workflow_checkpoint_contract_app_error;

use super::super::super::contracts::GALGAME_PIPELINE_MAX_TOKENS_PROFILE;
use super::{GalgameLlmOnceInput, PreparedGalgameLlmOnce};

pub(super) async fn prepare_galgame_llm_once(
    input: &GalgameLlmOnceInput<'_>,
) -> Result<PreparedGalgameLlmOnce, AppError> {
    let system_prompt = input
        .prompt_renderer
        .render_prompt(input.prompt_name, &HashMap::new(), Some(input.project_id))
        .await?;
    let fingerprint_json = json!({
        "workflow": input.workflow,
        "step_key": input.step_key.as_str(),
        "prompt_name": input.prompt_name,
        "system_prompt": system_prompt,
        "user_prompt": input.user_prompt.as_str(),
        "temperature": 0.25,
        "max_tokens_profile": GALGAME_PIPELINE_MAX_TOKENS_PROFILE,
    });
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    Ok(PreparedGalgameLlmOnce {
        system_prompt,
        fingerprint_json,
        input_fingerprint_sha256,
    })
}
