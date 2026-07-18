use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_db::workflow;
use a4n_llm::provider::ChatMessage;
use serde_json::json;

use crate::workflow::workflow_checkpoint_contract_app_error;

use super::super::super::contracts::{
    GALGAME_PIPELINE_MAX_TOKENS_PROFILE, GALGAME_PIPELINE_WORKFLOW,
};
use super::{GalgameLlmStageInput, PreparedGalgameLlmStage};

pub(super) async fn prepare_galgame_llm_stage(
    input: &GalgameLlmStageInput<'_>,
) -> Result<PreparedGalgameLlmStage, AppError> {
    let system_prompt = input
        .prompt_renderer
        .render_prompt(input.prompt_name, &HashMap::new(), Some(input.project_id))
        .await?;
    let request_messages = vec![
        ChatMessage::system(system_prompt.clone()),
        ChatMessage::user(input.user_prompt.clone()),
    ];
    let fingerprint_json = json!({
        "workflow": GALGAME_PIPELINE_WORKFLOW,
        "stage": input.stage.key(),
        "checkpoint_key": input.checkpoint_key,
        "prompt_name": input.prompt_name,
        "system_prompt": system_prompt,
        "user_prompt": input.user_prompt,
        "temperature": 0.25,
        "max_tokens_profile": GALGAME_PIPELINE_MAX_TOKENS_PROFILE,
    });
    let input_fingerprint_sha256 =
        workflow::llm_call_checkpoint_fingerprint_sha256(&fingerprint_json)
            .map_err(workflow_checkpoint_contract_app_error)?;
    Ok(PreparedGalgameLlmStage {
        request_messages,
        fingerprint_json,
        input_fingerprint_sha256,
    })
}
