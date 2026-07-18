use serde_json::Value;

use crate::actions::workflow_checkpoint::{
    CompletedLlmCallCheckpointInput, required_llm_call_checkpoint_json,
};
use crate::state::AppState;

use super::super::super::types::SceneProfileStageCheckpoint;

pub(super) async fn load_completed_scene_profile_stage_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    call_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<SceneProfileStageCheckpoint>, String> {
    let Some(checkpoint) = state
        .load_completed_llm_call_checkpoint(workflow_run_id, call_key, input_fingerprint_json)
        .await
        .map_err(|err| err.to_string())?
    else {
        return Ok(None);
    };
    let checkpoint_json =
        required_llm_call_checkpoint_json(checkpoint, call_key).map_err(|err| err.to_string())?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{call_key}.response_content 必须是字符串"))?;
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| format!("{call_key}.parsed_output 缺失"))?;
    Ok(Some(SceneProfileStageCheckpoint {
        response_content,
        parsed_output,
    }))
}

pub(super) async fn save_completed_scene_profile_stage_checkpoint(
    state: &AppState,
    workflow_run_id: i64,
    call_key: String,
    call_order: i32,
    input_fingerprint_json: Value,
    response_content: &str,
    parsed_output: Value,
) -> Result<(), String> {
    state
        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key,
            call_order,
            input_fingerprint_json,
            checkpoint_json: serde_json::json!({
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
