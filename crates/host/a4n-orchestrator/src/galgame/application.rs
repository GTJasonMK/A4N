use a4n_api_contract::galgame::{GalgamePipelineStatusSummary, GalgameProgressReporter};
use a4n_db_models::models::generation::workflow_run;
use a4n_domain::project::ProjectKind;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;

use crate::{
    OrchestratorContext,
    workflow::{ListWorkflowRunsInput, WORKFLOW_STATUS_RUNNING},
};
use a4n_domain::collaboration::ProjectAccessRole;

use super::{
    GALGAME_PIPELINE_WORKFLOW_KIND, GalgameAssetPlan, GalgamePipelineRunOptions,
    GalgamePipelineStartStage, GalgameScriptDetail, galgame_pipeline_workflow_run_input,
};

pub struct GalgamePipelineApplicationInput {
    pub project_id: i64,
    pub task_id: String,
    pub start_from_stage: Option<GalgamePipelineStartStage>,
    pub resume_from_checkpoint: bool,
    pub stop_after: Option<a4n_api_contract::galgame::GalgamePipelineStopAfter>,
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
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        self.find_script_detail(project_id).await
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
        input: super::RunGalgameAssetGenerationInput,
    ) -> Result<super::GalgameAssetGenerationSummary, AppError> {
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
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.run_galgame_pipeline_validated(input, prompt_renderer)
            .await
    }

    pub async fn prepare_galgame_pipeline_preview_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        start_from_stage: Option<GalgamePipelineStartStage>,
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
    ) -> Result<super::GalgameLine, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        super::read_models::galgame_line_from_model(
            self.insert_galgame_line(project_id, input).await?,
        )
    }

    pub async fn update_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::UpdateGalgameLineInput,
    ) -> Result<super::GalgameLine, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        super::read_models::galgame_line_from_model(
            self.update_galgame_line(project_id, input).await?,
        )
    }

    pub async fn update_galgame_scene_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::UpdateGalgameSceneInput,
    ) -> Result<super::GalgameScene, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        super::read_models::galgame_scene_from_model(
            self.update_galgame_scene(project_id, input).await?,
        )
    }

    pub async fn update_galgame_choices_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::UpdateGalgameChoicesInput,
    ) -> Result<Vec<super::GalgameChoice>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        Ok(self
            .update_galgame_choices(project_id, input)
            .await?
            .into_iter()
            .map(super::read_models::galgame_choice_from_model)
            .collect())
    }

    pub async fn reorder_galgame_lines_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: super::ReorderGalgameLinesInput,
    ) -> Result<Vec<super::GalgameLine>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.reorder_galgame_lines(project_id, input)
            .await?
            .into_iter()
            .map(super::read_models::galgame_line_from_model)
            .collect()
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
    ) -> Result<super::GalgameMediaBindingSummary, AppError> {
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
        host_port: &dyn a4n_api_contract::media::MediaAudioProbeHostPort,
        user_id: i64,
        input: super::ImportGalgameAudioInput,
    ) -> Result<super::GalgameMediaBindingSummary, AppError> {
        self.verify_galgame_project_access(user_id, input.project_id, ProjectAccessRole::Editor)
            .await?;
        self.import_galgame_audio_validated(host_port, input).await
    }

    pub async fn run_export_galgame_webgal_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<a4n_api_contract::export::ExportFileData, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_galgame_webgal_validated(project_id).await
    }

    pub async fn get_galgame_pipeline_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgamePipelineStatusSummary, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let (project_result, runs_result) = futures::join!(
            self.project_by_id(project_id),
            self.workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: None,
                chapter_ids: None,
                workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
                status: None,
                resumable_only: Some(false),
                limit: Some(1),
            }),
        );
        let project = project_result?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != ProjectKind::Galgame.as_str() {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {}",
                project.id
            )));
        }
        let run = runs_result?.into_iter().next();
        let checkpoints = match &run {
            Some(run) => self.workflow_checkpoint_models(run.id).await?,
            None => Vec::new(),
        };
        Ok(super::read_models::galgame_pipeline_status_summary(
            run,
            checkpoints,
        ))
    }

    pub async fn run_galgame_pipeline_validated(
        &self,
        input: GalgamePipelineApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Option<GalgameScriptDetail>, AppError> {
        if input.stop_after
            == Some(a4n_api_contract::galgame::GalgamePipelineStopAfter::BranchDesign)
            && input.start_from_stage.is_some_and(|stage| {
                stage.order() > GalgamePipelineStartStage::BranchDesign.order()
            })
        {
            return Err(AppError::InvalidArgument("开始阶段不能晚于结束阶段".into()));
        }
        let project = self
            .project_by_id(input.project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {}", input.project_id)))?;
        if project.project_kind != ProjectKind::Galgame.as_str() {
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
        let initial_stage = input
            .start_from_stage
            .unwrap_or(GalgamePipelineStartStage::AdaptationPlan);
        let workflow_input = galgame_pipeline_workflow_run_input(
            project.id,
            root_project_id,
            source_snapshot_id,
            initial_stage.key(),
            Some(input.task_id.clone()),
        )
        .map_err(AppError::InvalidArgument)?;
        let workflow_run = if input.resume_from_checkpoint {
            self.begin_or_resume_workflow_run_validated(workflow_input)
                .await?
        } else {
            self.begin_workflow_run_validated(workflow_input).await?
        };
        let result = self
            .run_pipeline(
                project.id,
                prompt_renderer,
                GalgamePipelineRunOptions {
                    workflow_run_id: workflow_run.id,
                    task_id: Some(input.task_id),
                    progress_reporter: input.progress_reporter,
                    resume_from_checkpoint: input.resume_from_checkpoint,
                    start_from_stage: input.start_from_stage,
                    stop_after: input.stop_after,
                },
            )
            .await;
        self.finalize_workflow_result(workflow_run.id, result, "Galgame 管线")
            .await
    }

    pub async fn cancel_galgame_pipeline_validated(
        &self,
        project_id: i64,
    ) -> Result<workflow_run::Model, AppError> {
        let (project_result, runs_result) = futures::join!(
            self.project_by_id(project_id),
            self.workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: None,
                chapter_ids: None,
                workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
                status: Some(WORKFLOW_STATUS_RUNNING.to_string()),
                resumable_only: Some(false),
                limit: Some(1),
            }),
        );
        let project = project_result?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != ProjectKind::Galgame.as_str() {
            return Err(AppError::InvalidArgument(format!(
                "项目不是 Galgame 派生项目: {}",
                project.id
            )));
        }
        let run = runs_result?
            .into_iter()
            .next()
            .ok_or_else(|| AppError::InvalidState {
                current: "no_running_galgame_pipeline".to_string(),
                operation: "取消 Galgame 管线".to_string(),
            })?;
        self.cancel_workflow_run_status_validated(run.id, "用户请求取消 Galgame 管线")
            .await
    }
}
