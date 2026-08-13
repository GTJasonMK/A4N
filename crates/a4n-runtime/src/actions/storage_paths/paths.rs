use super::StoragePathsSummary;
use crate::AppState;
use crate::actions::admin_records::{AdminPermission, require_admin_permission};
use a4n_core::config::app_config::portable_home_dir;
use a4n_core::error::AppError;
use std::path::{Path, PathBuf};

pub fn data_dir_path(data_dir: &Path) -> PathBuf {
    data_dir.to_path_buf()
}

pub fn log_dir_path(state: &AppState) -> PathBuf {
    state.log_dir().to_path_buf()
}

pub fn frontend_state_file_path(state: &AppState) -> PathBuf {
    state.app_paths().frontend_state_file()
}

pub async fn get_storage_paths(
    state: &AppState,
    caller_user_id: i64,
) -> Result<StoragePathsSummary, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    let paths = state.app_paths();
    Ok(StoragePathsSummary {
        application_home: path_to_string(portable_home_dir()),
        data_dir: path_to_string(&paths.data_dir),
        config_file: path_to_string(&paths.config_file),
        database_url: state.configured_database_url().to_string(),
        projects_dir: path_to_string(paths.data_dir.join("projects")),
        exports_dir: path_to_string(paths.exports_dir()),
        backups_dir: path_to_string(paths.backups_dir()),
        cache_dir: path_to_string(&paths.cache_dir),
        embedding_models_dir: path_to_string(
            crate::dynamic_embedding::effective_fastembed_cache_dir(
                state.default_fastembed_cache_dir(),
            ),
        ),
        temp_dir: path_to_string(&paths.temp_dir),
        log_dir: path_to_string(&paths.log_dir),
        webview_dir: path_to_string(paths.webview_dir()),
        frontend_state_file: path_to_string(paths.frontend_state_file()),
        prompts_dir: path_to_string(&paths.prompts_dir),
        prompts_git_dir: path_to_string(paths.data_dir.join("prompts.git")),
        qdrant_url: std::env::var("A4N_QDRANT_URL").unwrap_or_else(|_| String::new()),
        qdrant_collection: std::env::var("A4N_QDRANT_COLLECTION")
            .unwrap_or_else(|_| "a4n_rag".to_string()),
    })
}

pub(super) fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}
