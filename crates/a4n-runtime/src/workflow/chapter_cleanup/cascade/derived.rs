use a4n_core::error::AppError;

use crate::actions::app_error_message;
use crate::state::AppState;

use super::super::finalization::finish_chapter_derived_cleanup;
use super::validation::validate_chapter_media_file_paths;

pub async fn cleanup_chapter_derived_data(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    include_generated_outputs: bool,
) -> Result<(), String> {
    let artifacts = state
        .orchestrator
        .cleanup_chapter_derived_data_record(
            project_id,
            chapter_id,
            chapter_number,
            include_generated_outputs,
            |artifacts| {
                validate_chapter_media_file_paths(
                    &state.config.data_dir,
                    &artifacts.media_file_paths,
                )
                .map_err(|error| AppError::InvalidState {
                    current: error,
                    operation: "清理章节派生媒体文件".to_string(),
                })
            },
        )
        .await
        .map_err(app_error_message)?;
    finish_chapter_derived_cleanup(state, project_id, chapter_id, chapter_number, artifacts)
        .await?;
    Ok(())
}
