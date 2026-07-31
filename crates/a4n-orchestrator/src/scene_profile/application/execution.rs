mod chapters;
mod persistence;

use a4n_core::error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;

use super::{
    SceneProfileSyncHostPort,
    context::select_scene_profile_sync_chapters,
    progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress},
    workflow::finalize_scene_profile_sync_workflow,
};

impl OrchestratorContext {
    pub async fn scene_profile_sync_task_scope_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<super::types::SceneProfileSyncTaskScope, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(super::types::SceneProfileSyncTaskScope {
            project_id,
            total: super::progress::SCENE_PROFILE_SYNC_PROGRESS_TOTAL,
        })
    }

    pub async fn sync_scene_profiles_for_chapters_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: Option<i32>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
    ) -> Result<Value, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.sync_scene_profiles_for_chapters_validated(
            project_id,
            chapter_number,
            prompt_renderer,
            host_port,
            progress,
        )
        .await
    }

    pub async fn sync_scene_profiles_for_chapters_validated(
        &self,
        project_id: i64,
        chapter_number: Option<i32>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
    ) -> Result<Value, AppError> {
        report_scene_profile_sync_progress(progress, 0, "正在读取可同步章节", "场景档案同步已停止")
            .map_err(scene_profile_sync_application_error)?;
        let chapters = select_scene_profile_sync_chapters(self, project_id, chapter_number)
            .await
            .map_err(scene_profile_sync_application_error)?;
        let workflow_run = self
            .begin_scene_profile_sync_workflow_validated(
                project_id,
                chapter_number,
                &chapters,
                progress.and_then(|progress| progress.active_task_id),
            )
            .await?;
        let result = chapters::sync_scene_profile_chapters(
            self,
            prompt_renderer,
            host_port,
            project_id,
            workflow_run.id,
            chapters,
            progress,
        )
        .await;
        let finalized = finalize_scene_profile_sync_workflow(self, workflow_run.id, &result).await;
        if matches!(&finalized, Err(AppError::Cancelled)) {
            return Err(AppError::Cancelled);
        }
        match (result, finalized) {
            (Ok(summary), Ok(())) => Ok(summary),
            (Err(error), Ok(())) => Err(scene_profile_sync_application_error(error)),
            (Ok(_), Err(status_error)) => Err(AppError::InvalidState {
                current: "场景档案同步业务已完成".to_string(),
                operation: format!("写入 workflow completed 终态失败: {status_error}"),
            }),
            (Err(error), Err(status_error)) => Err(AppError::InvalidState {
                current: error,
                operation: format!(
                    "场景档案同步主流程失败，且 workflow failed 终态写入失败: {status_error}"
                ),
            }),
        }
    }
}

fn scene_profile_sync_application_error(message: String) -> AppError {
    if message == "场景档案同步已停止" {
        return AppError::Cancelled;
    }
    AppError::InvalidState {
        current: "scene_profile_sync".to_string(),
        operation: message,
    }
}
