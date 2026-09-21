use super::StoragePathsSummary;
use crate::AppState;
use crate::actions::admin_records::{AdminPermission, require_admin_permission};
use a4n_config::app_config::portable_home_dir;
use a4n_config::optional_non_empty_env_string;
use a4n_error::AppError;
use a4n_rag_core::DEFAULT_QDRANT_COLLECTION;
use a4n_storage::paths::AppPaths;
use std::path::{Path, PathBuf};

pub fn log_dir_path(state: &AppState) -> PathBuf {
    state.log_dir().to_path_buf()
}

pub async fn get_storage_paths(
    state: &AppState,
    caller_user_id: i64,
) -> Result<StoragePathsSummary, AppError> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings).await?;
    let config = state.host_app_config();
    let paths = AppPaths::new(
        config.data_dir.clone(),
        config.cache_dir.clone(),
        config.temp_dir.clone(),
        config.log_dir.clone(),
    );
    let qdrant_url = optional_non_empty_env_string("A4N_QDRANT_URL")?.unwrap_or_default();
    let qdrant_collection = optional_non_empty_env_string("A4N_QDRANT_COLLECTION")?
        .unwrap_or_else(|| DEFAULT_QDRANT_COLLECTION.to_string());
    Ok(StoragePathsSummary {
        application_home: path_to_string(portable_home_dir()),
        data_dir: path_to_string(&paths.data_dir),
        config_file: path_to_string(&paths.config_file),
        database_url: config.database_url.clone(),
        projects_dir: path_to_string(paths.data_dir.join("projects")),
        exports_dir: path_to_string(paths.exports_dir()),
        backups_dir: path_to_string(paths.backups_dir()),
        cache_dir: path_to_string(&paths.cache_dir),
        embedding_models_dir: path_to_string(state.embedding_models_dir()),
        temp_dir: path_to_string(&paths.temp_dir),
        log_dir: path_to_string(&paths.log_dir),
        webview_dir: path_to_string(paths.webview_dir()),
        frontend_state_file: path_to_string(paths.frontend_state_file()),
        prompts_git_dir: path_to_string(paths.data_dir.join("prompts.git")),
        qdrant_url,
        qdrant_collection,
    })
}

pub(super) fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}
