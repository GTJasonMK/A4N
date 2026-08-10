//! Galgame 领域 runtime 窄 facade。

use a4n_api_contract::export::ExportFileData;
use a4n_api_contract::galgame::{
    BindGalgameMediaInput, GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameChoice,
    GalgameLine, GalgameMediaBindingSummary, GalgameScene, GalgameScriptDetail,
    ImportGalgameAudioInput, InsertGalgameLineInput, ReorderGalgameLinesInput,
    RunGalgameAssetGenerationInput, UnbindGalgameMediaInput, UpdateGalgameChoicesInput,
    UpdateGalgameLineInput, UpdateGalgameSceneInput,
};
use a4n_core::error::AppError;
use a4n_domain::galgame::GalgamePipelineStartStage;
use a4n_orchestrator::{
    OrchestratorContext,
    galgame::{
        GalgameAssetGenerationHostPort, GalgamePipelineApplicationInput,
        GalgamePipelineStatusReadModel,
    },
};
use a4n_prompt::rendering::PromptRenderer;

pub(crate) struct GalgameRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> GalgameRuntimeFacade<'a> {
    pub(crate) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn get_galgame_script_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgameScriptDetail, AppError> {
        self.orchestrator
            .get_galgame_script_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn plan_galgame_assets_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgameAssetPlan, AppError> {
        self.orchestrator
            .plan_galgame_assets_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn run_galgame_asset_generation_for_user_validated(
        &self,
        host_port: &dyn GalgameAssetGenerationHostPort,
        user_id: i64,
        input: RunGalgameAssetGenerationInput,
    ) -> Result<GalgameAssetGenerationSummary, AppError> {
        self.orchestrator
            .run_galgame_asset_generation_for_user_validated(host_port, user_id, input)
            .await
    }

    pub(crate) async fn run_galgame_pipeline_for_user_validated(
        &self,
        user_id: i64,
        input: GalgamePipelineApplicationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<GalgameScriptDetail, AppError> {
        self.orchestrator
            .run_galgame_pipeline_for_user_validated(user_id, input, prompt_renderer)
            .await
    }

    pub(crate) async fn get_galgame_pipeline_status_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<GalgamePipelineStatusReadModel, AppError> {
        self.orchestrator
            .get_galgame_pipeline_status_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn cancel_galgame_pipeline_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        self.orchestrator
            .cancel_galgame_pipeline_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn insert_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: InsertGalgameLineInput,
    ) -> Result<GalgameLine, AppError> {
        self.orchestrator
            .insert_galgame_line_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn update_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameLineInput,
    ) -> Result<GalgameLine, AppError> {
        self.orchestrator
            .update_galgame_line_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn update_galgame_scene_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameSceneInput,
    ) -> Result<GalgameScene, AppError> {
        self.orchestrator
            .update_galgame_scene_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn update_galgame_choices_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameChoicesInput,
    ) -> Result<Vec<GalgameChoice>, AppError> {
        self.orchestrator
            .update_galgame_choices_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn reorder_galgame_lines_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: ReorderGalgameLinesInput,
    ) -> Result<Vec<GalgameLine>, AppError> {
        self.orchestrator
            .reorder_galgame_lines_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn delete_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        line_id: i64,
    ) -> Result<(), AppError> {
        self.orchestrator
            .delete_galgame_line_for_user_validated(user_id, project_id, line_id)
            .await
    }

    pub(crate) async fn prepare_galgame_pipeline_preview_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        start_from_stage: Option<GalgamePipelineStartStage>,
        resume_from_checkpoint: bool,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<a4n_orchestrator::galgame::GalgamePipelinePreviewRequest>, AppError> {
        self.orchestrator
            .prepare_galgame_pipeline_preview_for_user_validated(
                user_id,
                project_id,
                start_from_stage,
                resume_from_checkpoint,
                prompt_renderer,
            )
            .await
    }

    pub(crate) async fn run_export_galgame_webgal_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.orchestrator
            .run_export_galgame_webgal_for_user_validated(user_id, project_id)
            .await
    }

    pub(crate) async fn bind_galgame_media_for_user_validated(
        &self,
        user_id: i64,
        input: BindGalgameMediaInput,
    ) -> Result<GalgameMediaBindingSummary, AppError> {
        self.orchestrator
            .bind_galgame_media_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn unbind_galgame_media_for_user_validated(
        &self,
        user_id: i64,
        input: UnbindGalgameMediaInput,
    ) -> Result<u64, AppError> {
        self.orchestrator
            .unbind_galgame_media_for_user_validated(user_id, input)
            .await
    }

    pub(crate) async fn import_galgame_audio_for_user_validated(
        &self,
        user_id: i64,
        input: ImportGalgameAudioInput,
    ) -> Result<GalgameMediaBindingSummary, AppError> {
        self.orchestrator
            .import_galgame_audio_for_user_validated(user_id, input)
            .await
    }
}
