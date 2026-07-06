use crate::AppState;
use a4n_core::models::generation::workflow_checkpoint;
use a4n_services::generation::generation_checkpoint_fingerprint_sha256;
use a4n_services::workflow::{UpsertWorkflowCheckpointInput, WORKFLOW_STATUS_COMPLETED};
use serde_json::Value;

pub(crate) fn workflow_checkpoint_fingerprint_sha256(value: &Value) -> Result<String, String> {
    generation_checkpoint_fingerprint_sha256(value).map_err(|err| err.to_string())
}

pub(crate) async fn load_completed_workflow_checkpoint(
    state: &AppState,
    workflow_run_id: Option<i64>,
    step_key: &str,
    input_fingerprint_json: &Value,
) -> Result<Option<workflow_checkpoint::Model>, String> {
    let Some(workflow_run_id) = workflow_run_id else {
        return Ok(None);
    };
    let fingerprint_sha = workflow_checkpoint_fingerprint_sha256(input_fingerprint_json)?;
    state
        .workflow_run
        .completed_checkpoint_by_fingerprint(workflow_run_id, step_key, &fingerprint_sha)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) fn required_checkpoint_json(
    checkpoint: &workflow_checkpoint::Model,
    step_key: &str,
) -> Result<Value, String> {
    checkpoint
        .checkpoint_json
        .clone()
        .ok_or_else(|| format!("{step_key} 已完成 checkpoint 缺少 checkpoint_json"))
}

pub(crate) fn required_checkpoint_field(
    checkpoint_json: &Value,
    step_key: &str,
    field: &str,
) -> Result<Value, String> {
    checkpoint_json
        .get(field)
        .cloned()
        .ok_or_else(|| format!("{step_key} 已完成 checkpoint 缺少 {field}"))
}

pub(crate) async fn save_completed_workflow_checkpoint(
    state: &AppState,
    workflow_run_id: Option<i64>,
    step_key: impl Into<String>,
    step_order: i32,
    input_fingerprint_json: Value,
    checkpoint_json: Value,
) -> Result<(), String> {
    let Some(workflow_run_id) = workflow_run_id else {
        return Ok(());
    };
    let input_fingerprint_sha256 = workflow_checkpoint_fingerprint_sha256(&input_fingerprint_json)?;
    state
        .workflow_run
        .upsert_checkpoint(UpsertWorkflowCheckpointInput {
            workflow_run_id,
            step_key: step_key.into(),
            step_order,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input_fingerprint_json),
            checkpoint_json: Some(checkpoint_json),
            artifacts_json: None,
            generation_run_ids_json: None,
            error_message: None,
            completed: true,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
