use super::{OrphanProjectDataDirsSummary, scanning};
use crate::{
    AppState,
    actions::{
        admin_records::{AdminPermission, require_admin_permission},
        app_error_message,
    },
    file_utils::remove_data_dir_in_data_dir,
};
use a4n_core::files::paths::ProjectPaths;
use std::collections::HashSet;

pub async fn list_orphan_project_data_dirs(
    state: &AppState,
    caller_user_id: i64,
) -> Result<OrphanProjectDataDirsSummary, String> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|error| error.to_string())?;
    let existing_project_ids = existing_project_ids(state.orchestrator()).await?;
    scanning::orphan_project_data_dirs(state.data_dir(), &existing_project_ids)
}

pub async fn cleanup_orphan_project_data_dirs(
    state: &AppState,
    caller_user_id: i64,
) -> Result<OrphanProjectDataDirsSummary, String> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|error| error.to_string())?;
    remove_unowned_project_data_dirs(state).await
}

async fn remove_unowned_project_data_dirs(
    state: &AppState,
) -> Result<OrphanProjectDataDirsSummary, String> {
    let mut protected_project_ids = existing_project_ids(state.orchestrator()).await?;
    protected_project_ids.extend(
        state
            .orchestrator()
            .project_ids_with_incomplete_cleanup()
            .await
            .map_err(app_error_message)?,
    );
    let summary = scanning::orphan_project_data_dirs(state.data_dir(), &protected_project_ids)?;
    for item in &summary.items {
        if state
            .orchestrator()
            .project_by_id(item.project_id)
            .await
            .map_err(app_error_message)?
            .is_some()
        {
            return Err(format!(
                "孤儿目录清理期间项目已重新出现，拒绝删除: project_id={}",
                item.project_id
            ));
        }
        let prefix = crate::cleanup_outbox::project_media_path_prefix(item.project_id);
        if state
            .orchestrator()
            .media_path_prefix_has_persisted_reference(&prefix)
            .await
            .map_err(app_error_message)?
        {
            return Err(format!(
                "孤儿项目目录仍有持久化媒体引用，拒绝删除: {prefix}"
            ));
        }
    }
    for item in &summary.items {
        let project_paths = ProjectPaths::new(state.data_dir(), item.project_id);
        remove_data_dir_in_data_dir(state.data_dir(), project_paths.root(), "孤儿项目数据目录")?;
    }
    Ok(summary)
}

async fn existing_project_ids(
    project_read: &a4n_orchestrator::OrchestratorContext,
) -> Result<HashSet<i64>, String> {
    let ids = project_read
        .project_ids()
        .await
        .map_err(app_error_message)?;
    Ok(ids.into_iter().collect())
}
