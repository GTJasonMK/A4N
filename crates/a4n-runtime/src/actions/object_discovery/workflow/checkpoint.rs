use super::super::progress::OBJECT_DISCOVERY_CALL_KEY;
use super::super::types::ObjectDiscoveryCallCheckpoint;
use crate::actions::workflow_checkpoint::{
    CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
};
use crate::state::AppState;
use serde_json::{Value, json};

pub(in crate::actions::object_discovery) async fn load_completed_object_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: &Value,
) -> Result<Option<ObjectDiscoveryCallCheckpoint>, String> {
    let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(
            workflow_run_id,
            OBJECT_DISCOVERY_CALL_KEY,
            input_fingerprint_json,
        )
        .await
        .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, OBJECT_DISCOVERY_CALL_KEY)
        .map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| {
            format!(
                "{}.response_content 必须是字符串",
                OBJECT_DISCOVERY_CALL_KEY
            )
        })?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| format!("{}.parsed_output 缺失", OBJECT_DISCOVERY_CALL_KEY))?;
    Ok(Some(ObjectDiscoveryCallCheckpoint {
        response_content,
        parsed_output,
    }))
}

pub(in crate::actions::object_discovery) async fn save_completed_object_discovery_call_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), String> {
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: OBJECT_DISCOVERY_CALL_KEY.to_string(),
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
