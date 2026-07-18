use super::StoragePathsSummary;
use crate::AppState;
use a4n_core::{config::app_config::portable_home_dir, files::paths::AppPaths};
use std::path::{Path, PathBuf};

pub fn data_dir_path(state: &AppState) -> PathBuf {
    state.config.data_dir.clone()
}

pub fn log_dir_path(state: &AppState) -> PathBuf {
    state.config.log_dir.clone()
}

pub fn frontend_state_file_path(state: &AppState) -> PathBuf {
    AppPaths::from_config(&state.config).frontend_state_file()
}

pub fn get_storage_paths(state: &AppState) -> StoragePathsSummary {
    let paths = AppPaths::from_config(&state.config);
    StoragePathsSummary {
        application_home: path_to_string(portable_home_dir()),
        data_dir: path_to_string(&paths.data_dir),
        config_file: path_to_string(&paths.config_file),
        database_url: state.config.database_url.clone(),
        projects_dir: path_to_string(paths.data_dir.join("projects")),
        exports_dir: path_to_string(paths.exports_dir()),
        backups_dir: path_to_string(paths.backups_dir()),
        cache_dir: path_to_string(&paths.cache_dir),
        embedding_models_dir: path_to_string(
            crate::dynamic_embedding::effective_fastembed_cache_dir(
                crate::dynamic_embedding::default_fastembed_cache_dir(&state.config),
            ),
        ),
        temp_dir: path_to_string(&paths.temp_dir),
        log_dir: path_to_string(&paths.log_dir),
        webview_dir: path_to_string(paths.webview_dir()),
        frontend_state_file: path_to_string(paths.frontend_state_file()),
        prompts_dir: path_to_string(&paths.prompts_dir),
        prompts_git_dir: path_to_string(paths.data_dir.join("prompts.git")),
        lance_dir: path_to_string(paths.data_dir.join("lance")),
        tantivy_dir: path_to_string(paths.data_dir.join("tantivy")),
    }
}

pub(super) fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}
