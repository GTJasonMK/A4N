use a4n_core::{error::AppError, models::generation::workflow_run};
use a4n_prompt::rendering::PromptRenderer;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    workflow::{ListWorkflowRunsInput, WORKFLOW_STATUS_RUNNING, WorkflowRunFinalizeOutcome},
};

use super::{
    GALGAME_PIPELINE_WORKFLOW_KIND, GalgameAssetPlan, GalgamePipelineRunOptions,
    GalgamePipelineStartStage, GalgamePipelineStatusReadModel, GalgameProgressReporter,
    GalgameScriptDetail, PROJECT_KIND_GALGAME, galgame_pipeline_workflow_run_input,
    parse_galgame_start_stage,
};

pub struct GalgamePipelineApplicationInput {
    pub project_id: i64,
    pub task_id: Option<String>,
    pub start_from_stage: Option<String>,
    pub resume_from_checkpoint: bool,
    pub progress_reporter: Option<GalgameProgressReporter>,
}

impl OrchestratorContext {
    pub(crate) async fn verify_galgame_project_access(
        &self,
        user_id: i64,
        project_id: i64,
        role: ProjectAccessRole,
    ) -> Result<(), AppError> {
        self.verify_project_permission(project_id, user_id, role)
            .await
            .map(|_| ())
    }

    pub async fn get_galgame_script_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgameScriptDetail, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        self.get_script_detail(project_id).await
    }

    pub async fn plan_galgame_assets_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgameAssetPlan, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.plan_galgame_assets(project_id).await
    }

    pub async fn run_galgame_asset_generation_for_user_validated(
        &self,
        host_port: &dyn super::GalgameAssetGenerationHostPort,
        user_id: i64,
        input: super::GalgameAssetGenerationApplicationInput,
    ) -> Result<super::GalgameAssetGenerationApplicationSummary, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.run_galgame_asset_generation_validated(host_port, user_id, input)
            .await
    }

    pub async fn run_galgame_pipeline_for_user_validated(
        &self,
        user_id: i64,
        input: GalgamePipelineApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<GalgameScriptDetail, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.run_galgame_pipeline_validated(input, prompt_renderer)
            .await
    }

    pub async fn prepare_galgame_pipeline_preview_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        start_from_stage: Option<&str>,
        resume_from_checkpoint: bool,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<super::GalgamePipelinePreviewRequest>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.prepare_galgame_pipeline_preview_validated(
            project_id,
            start_from_stage,
            resume_from_checkpoint,
            prompt_renderer,
        )
        .await
    }

    pub async fn cancel_galgame_pipeline_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.cancel_galgame_pipeline_validated(project_id).await
    }

    pub async fn insert_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::InsertGalgameLineInput,
    ) -> Result<a4n_core::models::galgame::galgame_line::Model, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.insert_galgame_line(project_id, input).await
    }

    pub async fn update_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::UpdateGalgameLineInput,
    ) -> Result<a4n_core::models::galgame::galgame_line::Model, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.update_galgame_line(project_id, input).await
    }

    pub async fn update_galgame_scene_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::UpdateGalgameSceneInput,
    ) -> Result<a4n_core::models::galgame::galgame_scene::Model, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.update_galgame_scene(project_id, input).await
    }

    pub async fn update_galgame_choices_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::UpdateGalgameChoicesInput,
    ) -> Result<Vec<a4n_core::models::galgame::galgame_choice::Model>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.update_galgame_choices(project_id, input).await
    }

    pub async fn reorder_galgame_lines_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::ReorderGalgameLinesInput,
    ) -> Result<Vec<a4n_core::models::galgame::galgame_line::Model>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.reorder_galgame_lines(project_id, input).await
    }

    pub async fn delete_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        line_id: i64,
    ) -> Result<(), AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.delete_galgame_line(project_id, line_id).await
    }

    pub async fn bind_galgame_media_for_user_validated(
        &self,
        user_id: i64,
        input: super::BindGalgameMediaInput,
    ) -> Result<super::GalgameMediaBindingResult, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.bind_galgame_media_validated(input).await
    }

    pub async fn unbind_galgame_media_for_user_validated(
        &self,
        user_id: i64,
        input: super::UnbindGalgameMediaInput,
    ) -> Result<u64, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.unbind_galgame_media_validated(input).await
    }

    pub async fn import_galgame_audio_for_user_validated(
        &self,
        host_port: &dyn super::GalgameAudioHostPort,
        user_id: i64,
        input: super::ImportGalgameAudioInput,
    ) -> Result<super::GalgameMediaBindingResult, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.import_galgame_audio_validated(host_port, input).await
    }

    pub async fn run_export_galgame_webgal_for_user_validated(
        &self,
        host_port: &dyn super::WebgalExportHostPort,
        user_id: i64,
        project_id: i64,
    ) -> Result<crate::export::ExportFileData, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_galgame_webgal_validated(host_port, project_id)
            .await
    }

    pub async fn get_galgame_pipeline_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgamePipelineStatusReadModel, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_GALGAME {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {}",
                project.id
            )));
        }
        let run = self
            .workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project.id),
                workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
                status: None,
                resumable_only: None,
                limit: Some(1),
            })
            .await?
            .into_iter()
            .next();
        let checkpoints = match &run {
            Some(run) => self.workflow_checkpoint_models(run.id).await?,
            None => Vec::new(),
        };
        Ok(GalgamePipelineStatusReadModel { run, checkpoints })
    }

    pub async fn run_galgame_pipeline_validated(
        &self,
        input: GalgamePipelineApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<GalgameScriptDetail, AppError> {
        let project = self
            .project_by_id(input.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", input.project_id)))?;
        if project.project_kind != PROJECT_KIND_GALGAME {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {}",
                project.id
            )));
        }
        let source_snapshot_id =
            project
                .source_snapshot_id
                .ok_or_else(|| AppError::InvalidState {
                    current: "missing_source_snapshot".to_string(),
                    operation: format!("运行 Galgame 项目 {} 管线", project.id),
                })?;
        let root_project_id = project
            .root_project_id
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_root_project".to_string(),
                operation: format!("运行 Galgame 项目 {} 管线", project.id),
            })?;
        let start_from_stage = parse_galgame_start_stage(input.start_from_stage.as_deref())?;
        let initial_stage =
            start_from_stage.unwrap_or(GalgamePipelineStartStage::ShootingScriptProjection);
        let workflow_run = self
            .begin_workflow_run_validated(
                galgame_pipeline_workflow_run_input(
                    project.id,
                    root_project_id,
                    source_snapshot_id,
                    initial_stage.key(),
                    input.task_id.clone(),
                )
                .map_err(AppError::InvalidArgument)?,
            )
            .await?;
        let result = self
            .run_pipeline(
                project.id,
                prompt_renderer,
                GalgamePipelineRunOptions {
                    workflow_run_id: workflow_run.id,
                    task_id: input.task_id,
                    progress_reporter: input.progress_reporter,
                    resume_from_checkpoint: input.resume_from_checkpoint,
                    start_from_stage,
                },
            )
            .await;
        self.finalize_galgame_pipeline_application(workflow_run.id, result)
            .await
    }

    pub async fn cancel_galgame_pipeline_validated(
        &self,
        project_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_GALGAME {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {}",
                project.id
            )));
        }
        let run = self
            .workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project.id),
                workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
                status: Some(WORKFLOW_STATUS_RUNNING.to_string()),
                resumable_only: None,
                limit: Some(1),
            })
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::InvalidState {
                current: "no_running_galgame_pipeline".to_string(),
                operation: "取消 Galgame 管线".to_string(),
            })?;
        self.cancel_workflow_run_status_validated(run.id, Some("用户请求取消 Galgame 管线"))
            .await
    }

    async fn finalize_galgame_pipeline_application(
        &self,
        workflow_run_id: i64,
        result: Result<GalgameScriptDetail, AppError>,
    ) -> Result<GalgameScriptDetail, AppError> {
        let outcome = match &result {
            Ok(_) => WorkflowRunFinalizeOutcome::Completed,
            Err(AppError::Cancelled) => WorkflowRunFinalizeOutcome::Cancelled,
            Err(_) => WorkflowRunFinalizeOutcome::Failed,
        };
        let error_message = match &result {
            Err(AppError::Cancelled) => Some("Galgame 管线已取消".to_string()),
            Err(error) => Some(error.to_string()),
            Ok(_) => None,
        };
        let finalized = self
            .finalize_workflow_run_outcome_validated(
                workflow_run_id,
                outcome,
                error_message.as_deref(),
            )
            .await;
        match (result, finalized) {
            (Ok(detail), Ok(_)) => Ok(detail),
            (Err(error), Ok(_)) => Err(error),
            (Ok(_), Err(error)) => Err(AppError::InvalidState {
                current: "galgame_pipeline_completed".to_string(),
                operation: format!("Galgame 管线完成，但工作流终态写入失败: {error}"),
            }),
            (Err(error), Err(finalize_error)) => Err(AppError::InvalidState {
                current: error.to_string(),
                operation: format!("Galgame 管线失败，且工作流终态写入失败: {finalize_error}"),
            }),
        }
    }
}
