mod chapters;
mod persistence;

use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    scene_profile::{scene_profile_sync_business_key, scene_profile_sync_workflow_fingerprint},
};

use super::{
    SceneProfileSyncHostPort,
    context::select_scene_profile_sync_chapters,
    progress::{SceneProfileSyncProgress, report_scene_profile_sync_progress},
};

impl OrchestratorContext {
    pub async fn sync_scene_profiles_validated(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: Option<i32>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
        progress: Option<&SceneProfileSyncProgress<'_>>,
        workflow_run_id: Option<i64>,
    ) -> Result<(), AppError> {
        report_scene_profile_sync_progress(progress, 0, "正在读取可同步章节")?;
        let chapters = select_scene_profile_sync_chapters(self, project_id, chapter_number).await?;
        let (workflow_run_id, owns_workflow_run) = match workflow_run_id {
            Some(workflow_run_id) => {
                self.workflow_run_summary(workflow_run_id).await?;
                (workflow_run_id, false)
            }
            None => (
                self.begin_scene_profile_sync_workflow_validated(
                    project_id,
                    chapter_number,
                    &chapters,
                    progress.and_then(SceneProfileSyncProgress::active_task_id),
                )
                .await?
                .id,
                true,
            ),
        };
        let result = chapters::sync_scene_profile_chapters(
            self,
            user_id,
            prompt_renderer,
            host_port,
            project_id,
            workflow_run_id,
            chapters,
            progress,
        )
        .await;
        let result = match result {
            Ok(())
                if self
                    .workflow_run_summary(workflow_run_id)
                    .await?
                    .workflow_kind
                    != "chapter_analysis" =>
            {
                host_port
                    .sync_scene_rag_records(project_id, workflow_run_id)
                    .await
            }
            result => result,
        };
        if owns_workflow_run {
            self.finalize_workflow_result(workflow_run_id, result, "场景档案同步")
                .await
        } else {
            result
        }
    }

    pub(crate) async fn resume_scene_profile_sync_workflow_run_validated(
        &self,
        user_id: i64,
        requested_run: &a4n_db::workflow::WorkflowRunSummary,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn SceneProfileSyncHostPort,
    ) -> Result<(), AppError> {
        if requested_run.workflow_kind != "scene_profile_sync" {
            return Err(AppError::InvalidArgument(format!(
                "工作流类型 {} 不是场景档案同步",
                requested_run.workflow_kind
            )));
        }
        crate::workflow::require_workflow_can_resume(
            &requested_run.status,
            requested_run.can_resume,
            requested_run.resume_block_reason.as_deref(),
        )
        .map_err(|operation| AppError::InvalidState {
            current: requested_run.status.clone(),
            operation,
        })?;
        let project_id = requested_run.project_id.ok_or_else(|| {
            AppError::InvalidArgument("场景档案同步工作流缺少 project_id".to_string())
        })?;
        if requested_run.scope_table.as_deref() != Some("novel_project")
            || requested_run.scope_id != Some(project_id)
        {
            return Err(AppError::InvalidState {
                current: "scene_profile_sync_scope_invalid".to_string(),
                operation: "恢复场景档案同步".to_string(),
            });
        }
        let chapter_number = scene_profile_sync_chapter_number(requested_run)?;
        let chapters = select_scene_profile_sync_chapters(self, project_id, chapter_number).await?;
        let expected_fingerprint =
            scene_profile_sync_workflow_fingerprint(project_id, chapter_number, &chapters);
        if requested_run.input_fingerprint_json.as_ref() != Some(&expected_fingerprint) {
            return Err(AppError::InvalidState {
                current: "scene_profile_sync_input_changed".to_string(),
                operation: "恢复已被正文或分析结果取代的场景档案同步".to_string(),
            });
        }
        let latest = self
            .latest_workflow_run_by_business_key(&scene_profile_sync_business_key(
                project_id,
                chapter_number,
            ))
            .await?
            .ok_or_else(|| AppError::NotFound("场景档案同步工作流不存在".to_string()))?;
        if latest.id != requested_run.id {
            return Err(AppError::InvalidState {
                current: "scene_profile_sync_workflow_replaced".to_string(),
                operation: "恢复已被新运行取代的场景档案同步".to_string(),
            });
        }
        let running = self
            .mark_workflow_running_validated(requested_run.id)
            .await?;
        let result = self
            .sync_scene_profiles_validated(
                user_id,
                project_id,
                chapter_number,
                prompt_renderer,
                host_port,
                None,
                Some(running.id),
            )
            .await;
        self.finalize_workflow_result(running.id, result, "场景档案同步")
            .await
    }
}

fn scene_profile_sync_chapter_number(
    run: &a4n_db::workflow::WorkflowRunSummary,
) -> Result<Option<i32>, AppError> {
    let fingerprint =
        run.input_fingerprint_json
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: format!("scene_profile_sync_run_id={}", run.id),
                operation: "场景档案同步工作流缺少输入指纹".to_string(),
            })?;
    match fingerprint.get("chapter_number") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .and_then(|value| i32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| AppError::InvalidState {
                current: format!("scene_profile_sync_run_id={}", run.id),
                operation: "场景档案同步工作流 chapter_number 无效".to_string(),
            }),
    }
}
