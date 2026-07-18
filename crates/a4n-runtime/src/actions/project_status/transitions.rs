use crate::{
    AppState,
    actions::access::{verify_project_editor_string, verify_project_owner_string},
    file_utils::remove_data_file_in_data_dir,
};
use a4n_core::models::project::novel_project;

pub async fn forward_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: &str,
) -> Result<novel_project::Model, String> {
    verify_project_editor_string(state, user_id, project_id).await?;
    state
        .orchestrator
        .forward_project_status_record(project_id, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn rollback_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: &str,
) -> Result<novel_project::Model, String> {
    verify_project_owner_string(state, user_id, project_id).await?;
    let rollback = state
        .orchestrator
        .rollback_project_status_record(project_id, target)
        .await
        .map_err(|error| error.to_string())?;
    if rollback.needs_rag_clear {
        state
            .orchestrator
            .clear_project_rag(project_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    for path in rollback.media_paths {
        remove_data_file_in_data_dir(&state.config.data_dir, &path, "文件")?;
    }
    Ok(rollback.project)
}
