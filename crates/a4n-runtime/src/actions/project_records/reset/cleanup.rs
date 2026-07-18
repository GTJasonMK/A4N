use crate::AppState;
use crate::file_utils::remove_data_file_in_data_dir;

pub(super) async fn clear_rag_and_remove_media_files(
    state: &AppState,
    project_id: i64,
    media_paths: &[String],
) -> Result<(), String> {
    state
        .orchestrator
        .clear_project_rag(project_id)
        .await
        .map_err(|error| error.to_string())?;
    for path in media_paths {
        remove_data_file_in_data_dir(&state.config.data_dir, path, "文件")?;
    }
    Ok(())
}
