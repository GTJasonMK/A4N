use crate::AppState;
use crate::actions::admin_records::{AdminPermission, require_admin_permission};
use crate::file_utils::remove_data_dir_in_data_dir;
use a4n_core::config::app_config::portable_home_dir;
use a4n_core::files::paths::AppPaths;
use a4n_core::models::project::novel_project;
use sea_orm::{EntityTrait, QuerySelect};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct StoragePathsSummary {
    pub application_home: String,
    pub data_dir: String,
    pub config_file: String,
    pub database_url: String,
    pub projects_dir: String,
    pub exports_dir: String,
    pub backups_dir: String,
    pub cache_dir: String,
    pub embedding_models_dir: String,
    pub temp_dir: String,
    pub log_dir: String,
    pub webview_dir: String,
    pub frontend_state_file: String,
    pub prompts_dir: String,
    pub prompts_git_dir: String,
    pub lance_dir: String,
    pub tantivy_dir: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrphanProjectDataDirSummary {
    pub project_id: i64,
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrphanProjectDataDirsSummary {
    pub items: Vec<OrphanProjectDataDirSummary>,
    pub total_count: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CleanupOrphanProjectDataDirsInput {
    pub caller_user_id: i64,
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

pub async fn list_orphan_project_data_dirs(
    state: &AppState,
    caller_user_id: i64,
) -> Result<OrphanProjectDataDirsSummary, String> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|err| err.to_string())?;
    let existing_project_ids = existing_project_ids(state).await?;
    orphan_project_data_dirs(&state.config.data_dir, &existing_project_ids)
}

pub async fn cleanup_orphan_project_data_dirs(
    state: &AppState,
    input: CleanupOrphanProjectDataDirsInput,
) -> Result<OrphanProjectDataDirsSummary, String> {
    require_admin_permission(state, input.caller_user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|err| err.to_string())?;
    let existing_project_ids = existing_project_ids(state).await?;
    let summary = orphan_project_data_dirs(&state.config.data_dir, &existing_project_ids)?;
    for item in &summary.items {
        remove_data_dir_in_data_dir(
            &state.config.data_dir,
            &PathBuf::from(&item.path),
            "孤儿项目数据目录",
        )?;
    }
    Ok(summary)
}

async fn existing_project_ids(state: &AppState) -> Result<HashSet<i64>, String> {
    let ids = novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .into_tuple::<i64>()
        .all(state.db.as_ref())
        .await
        .map_err(|err| format!("查询项目列表失败：{err}"))?;
    Ok(ids.into_iter().collect())
}

fn orphan_project_data_dirs(
    data_dir: &Path,
    existing_project_ids: &HashSet<i64>,
) -> Result<OrphanProjectDataDirsSummary, String> {
    let projects_dir = data_dir.join("projects");
    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Ok(OrphanProjectDataDirsSummary {
                items: Vec::new(),
                total_count: 0,
                total_size_bytes: 0,
            });
        }
        Err(err) => return Err(format!("读取项目数据目录失败：{err}")),
    };

    let mut items = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取项目数据目录项失败：{err}"))?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        let Ok(project_id) = name.parse::<i64>() else {
            continue;
        };
        if project_id <= 0 || existing_project_ids.contains(&project_id) {
            continue;
        }
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|err| format!("读取项目数据目录元数据失败：{err}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        items.push(OrphanProjectDataDirSummary {
            project_id,
            path: path_to_string(&path),
            size_bytes: dir_size_without_following_symlinks(&path)?,
        });
    }
    items.sort_by_key(|item| item.project_id);
    let total_count = items.len();
    let total_size_bytes = items.iter().map(|item| item.size_bytes).sum();
    Ok(OrphanProjectDataDirsSummary {
        total_count,
        total_size_bytes,
        items,
    })
}

fn dir_size_without_following_symlinks(path: &Path) -> Result<u64, String> {
    let mut total = 0_u64;
    let entries = std::fs::read_dir(path).map_err(|err| format!("读取目录大小失败：{err}"))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取目录大小项失败：{err}"))?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|err| format!("读取目录大小元数据失败：{err}"))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            total = total.saturating_add(dir_size_without_following_symlinks(&path)?);
        } else {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn path_to_string(path: impl AsRef<std::path::Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .expect("system clock must be after UNIX_EPOCH");
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), stamp))
    }

    #[test]
    fn path_to_string_uses_lossy_platform_path_display() {
        assert_eq!(
            path_to_string(PathBuf::from("data/projects")),
            "data/projects"
        );
    }

    #[test]
    fn orphan_project_data_dirs_returns_numeric_dirs_missing_from_database() {
        let data_dir = unique_temp_dir("a4n_storage_orphans");
        let projects_dir = data_dir.join("projects");
        std::fs::create_dir_all(projects_dir.join("1")).expect("create live project dir");
        std::fs::create_dir_all(projects_dir.join("2/images")).expect("create orphan project dir");
        std::fs::write(projects_dir.join("2/images/ref.png"), b"demo").expect("write orphan file");
        std::fs::create_dir_all(projects_dir.join("notes")).expect("create nonnumeric dir");
        std::fs::write(projects_dir.join("3"), b"not a dir").expect("write numeric file");
        let existing = HashSet::from([1_i64]);

        let summary =
            orphan_project_data_dirs(&data_dir, &existing).expect("scan orphan project dirs");

        assert_eq!(summary.total_count, 1);
        assert_eq!(summary.items[0].project_id, 2);
        assert_eq!(summary.items[0].size_bytes, 4);

        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn orphan_project_data_dirs_accepts_missing_projects_dir() {
        let data_dir = unique_temp_dir("a4n_storage_no_projects");

        let summary =
            orphan_project_data_dirs(&data_dir, &HashSet::new()).expect("missing dir is empty");

        assert_eq!(summary.total_count, 0);
        assert!(summary.items.is_empty());

        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
