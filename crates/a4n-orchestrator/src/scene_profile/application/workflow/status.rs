use crate::OrchestratorContext;
use serde_json::Value;

pub(in crate::scene_profile::application) async fn finalize_scene_profile_sync_workflow(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    result: &Result<Value, String>,
) -> Result<(), String> {
    let (success, error_message) = match result {
        Ok(_) => (true, None),
        Err(message) => (false, Some(message.as_str())),
    };
    state
        .finalize_workflow_run_validated(workflow_run_id, success, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
