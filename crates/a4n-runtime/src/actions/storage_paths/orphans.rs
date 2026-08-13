use super::{OrphanProjectDataDirsSummary, scanning};
use crate::{
    AppState,
    actions::{
        admin_records::{AdminPermission, require_admin_permission},
        app_error_message,
    },
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::remove_data_dir_in_data_dir,
};
use std::{collections::HashSet, path::PathBuf};

pub(crate) async fn recover_unreferenced_managed_media_files(state: &AppState) {
    let files = match scanning::managed_media_files(state.data_dir()) {
        Ok(files) => files,
        Err(error) => {
            tracing::warn!(%error, "扫描强杀遗留媒体文件失败");
            return;
        }
    };
    if files.is_empty() {
        return;
    }
    let paths = files
        .iter()
        .map(|file| file.relative_path.clone())
        .collect::<Vec<_>>();
    let unreferenced = match state.orchestrator().unreferenced_media_paths(&paths).await {
        Ok(paths) => paths,
        Err(error) => {
            tracing::warn!(error = %app_error_message(error), "复核强杀遗留媒体文件引用失败");
            return;
        }
    };
    for path in unreferenced {
        let Ok(index) = files.binary_search_by(|file| file.relative_path.cmp(&path)) else {
            tracing::error!(media_path = %path, "DB 返回了扫描候选之外的未引用媒体路径");
            continue;
        };
        let file = &files[index];
        if let Err(error) = enqueue_file_artifact_cleanup_by_project(
            state,
            &format!("startup-orphan-media:{}", file.version),
            std::slice::from_ref(&path),
        )
        .await
        {
            tracing::warn!(
                %error,
                project_id = file.project_id,
                media_path = %path,
                "写入强杀遗留媒体 cleanup intent 失败"
            );
        }
    }
}

pub(crate) async fn recover_orphan_project_data_dirs(state: &AppState) {
    match remove_unowned_project_data_dirs(state).await {
        Ok(summary) if summary.total_count > 0 => tracing::info!(
            total_count = summary.total_count,
            total_size_bytes = summary.total_size_bytes,
            "启动恢复孤儿项目数据目录完成"
        ),
        Ok(_) => {}
        Err(error) => tracing::warn!(%error, "启动恢复孤儿项目数据目录失败"),
    }
}

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
        remove_data_dir_in_data_dir(
            state.data_dir(),
            &PathBuf::from(&item.path),
            "孤儿项目数据目录",
        )?;
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
