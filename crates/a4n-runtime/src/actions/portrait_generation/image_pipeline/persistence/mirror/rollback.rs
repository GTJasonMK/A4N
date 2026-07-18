use crate::AppState;
use crate::file_utils::remove_data_file_in_data_dir;

pub(in crate::actions::portrait_generation) async fn rollback_failed_portrait_creation(
    state: &AppState,
    portrait_id: i64,
    image_path: &str,
) -> Result<(), String> {
    let rollback = state
        .orchestrator
        .rollback_failed_portrait_creation_records(portrait_id, image_path)
        .await
        .map_err(|err| err.to_string())?;
    if rollback.delete_file {
        remove_data_file_in_data_dir(&state.config.data_dir, image_path, "立绘图片")?;
    }
    Ok(())
}
