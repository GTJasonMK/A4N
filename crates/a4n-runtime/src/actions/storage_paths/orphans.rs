use super::{CleanupOrphanProjectDataDirsInput, OrphanProjectDataDirsSummary, scanning};
use crate::{
    AppState,
    actions::{
        admin_records::{AdminPermission, require_admin_permission},
        app_error_message,
    },
    file_utils::remove_data_dir_in_data_dir,
};
use std::{collections::HashSet, path::PathBuf};

pub async fn list_orphan_project_data_dirs(
    state: &AppState,
    caller_user_id: i64,
) -> Result<OrphanProjectDataDirsSummary, String> {
    require_admin_permission(state, caller_user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|error| error.to_string())?;
    let existing_project_ids = existing_project_ids(state).await?;
    scanning::orphan_project_data_dirs(&state.config.data_dir, &existing_project_ids)
}

pub async fn cleanup_orphan_project_data_dirs(
    state: &AppState,
    input: CleanupOrphanProjectDataDirsInput,
) -> Result<OrphanProjectDataDirsSummary, String> {
    require_admin_permission(state, input.caller_user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|error| error.to_string())?;
    let existing_project_ids = existing_project_ids(state).await?;
    let summary =
        scanning::orphan_project_data_dirs(&state.config.data_dir, &existing_project_ids)?;
    for item in &summary.items {
        if state
            .orchestrator
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
            .orchestrator
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
        remove_data_dir_in_data_dir(
            &state.config.data_dir,
            &PathBuf::from(&item.path),
            "孤儿项目数据目录",
        )?;
    }
    Ok(summary)
}

async fn existing_project_ids(state: &AppState) -> Result<HashSet<i64>, String> {
    let ids = state
        .orchestrator
        .project_ids()
        .await
        .map_err(app_error_message)?;
    Ok(ids.into_iter().collect())
}
