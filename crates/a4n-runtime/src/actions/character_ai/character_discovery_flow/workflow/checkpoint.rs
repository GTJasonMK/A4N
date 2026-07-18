use serde_json::{Value, json};

use crate::{
    AppState,
    actions::workflow_checkpoint::{
        CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
    },
};
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};

use super::super::contracts::{CHARACTER_DISCOVERY_CALL_KEY, CharacterDiscoveryCallCheckpoint};

pub(in crate::actions::character_ai::character_discovery_flow) async fn load_completed_character_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: &Value,
) -> Result<Option<CharacterDiscoveryCallCheckpoint>, String> {
    let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(
            workflow_run_id,
            CHARACTER_DISCOVERY_CALL_KEY,
            input_fingerprint_json,
        )
        .await
        .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_llm_call_checkpoint_json(checkpoint, CHARACTER_DISCOVERY_CALL_KEY)
            .map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{CHARACTER_DISCOVERY_CALL_KEY}.response_content 必须是字符串"))?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| format!("{CHARACTER_DISCOVERY_CALL_KEY}.parsed_output 缺失"))?;
    Ok(Some(CharacterDiscoveryCallCheckpoint {
        response_content,
        parsed_output,
    }))
}

pub(in crate::actions::character_ai::character_discovery_flow) async fn save_completed_character_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), String> {
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: CHARACTER_DISCOVERY_CALL_KEY.to_string(),
            call_order: 1,
            input_fingerprint_json,
            checkpoint_json: json!({
                "output": parsed_output.clone(),
                "parsed_output": parsed_output,
                "response_content": response_content,
                "usage": null,
                "finish_reason": null}),
            artifacts_json: None,
        })
        .await
        .map_err(|err| err.to_string())
}

pub(in crate::actions::character_ai::character_discovery_flow) async fn finalize_character_discovery_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Value, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
