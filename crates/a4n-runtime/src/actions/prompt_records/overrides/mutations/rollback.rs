use super::super::super::client::{sidecar_remove_project_override, sidecar_set_project_override};
use crate::AppState;
use a4n_orchestrator::prompt_override::ProjectOverrideIdentity;

pub(in crate::actions::prompt_records) async fn rollback_project_override_runtime_after_persist_error(
    engine: &dyn a4n_prompt::engine::PromptEngine,
    identity: &ProjectOverrideIdentity,
    previous_content: Option<String>,
    operation_error: String,
    rollback_success_message: &str,
    rollback_failure_prefix: &str,
) -> String {
    let rollback_result = match previous_content {
        Some(content) => {
            engine
                .set_project_override(identity.project_id, &identity.name, content)
                .await
        }
        None => {
            engine
                .remove_project_override(identity.project_id, &identity.name)
                .await
        }
    };
    rollback_message(
        rollback_result.map_err(|error| error.to_string()),
        operation_error,
        rollback_success_message,
        rollback_failure_prefix,
    )
}

pub(super) async fn rollback_project_override_sidecar_after_persist_error(
    state: &AppState,
    identity: &ProjectOverrideIdentity,
    previous_content: Option<String>,
    operation_error: String,
    rollback_success_message: &str,
    rollback_failure_prefix: &str,
) -> String {
    let rollback_result = match previous_content {
        Some(content) => {
            sidecar_set_project_override(state, identity.project_id, &identity.name, content).await
        }
        None => sidecar_remove_project_override(state, identity.project_id, &identity.name).await,
    };
    rollback_message(
        rollback_result,
        operation_error,
        rollback_success_message,
        rollback_failure_prefix,
    )
}

fn rollback_message(
    rollback_result: Result<(), String>,
    operation_error: String,
    rollback_success_message: &str,
    rollback_failure_prefix: &str,
) -> String {
    match rollback_result {
        Ok(()) => format!("{operation_error}；{rollback_success_message}"),
        Err(error) => format!("{operation_error}；{rollback_failure_prefix}{error}"),
    }
}
