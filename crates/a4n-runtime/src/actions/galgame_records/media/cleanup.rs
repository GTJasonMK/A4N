use crate::{AppState, file_utils::remove_data_file_in_data_dir};

pub(super) async fn cleanup_imported_media_asset(
    state: &AppState,
    media_asset_id: i64,
    file_path: &str,
) -> Result<(), String> {
    state
        .orchestrator
        .delete_media_asset_with_provenance_records(media_asset_id)
        .await
        .map_err(|err| format!("Galgame 音频媒体资产回滚失败：{err}"))?;
    remove_data_file_in_data_dir(&state.config.data_dir, file_path, "Galgame 音频文件")
}

pub(super) fn append_cleanup_error(primary: String, cleanup_error: Option<String>) -> String {
    match cleanup_error {
        Some(cleanup_error) if !cleanup_error.trim().is_empty() => {
            format!("{primary}；后续清理也失败：{cleanup_error}")
        }
        _ => primary,
    }
}
