//! Galgame 领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::galgame::{galgame_choice, galgame_line, galgame_scene};
use a4n_orchestrator::{
    OrchestratorContext,
    galgame::{
        BindGalgameMediaInput, GalgameAssetGenerationHostPort, GalgameAssetPlan,
        GalgameAudioHostPort, GalgameMediaBindingResult, GalgamePipelineApplicationInput,
        GalgamePipelineStatusReadModel, GalgameScriptDetail, ImportGalgameAudioInput,
        InsertGalgameLineInput, ReorderGalgameLinesInput, UnbindGalgameMediaInput,
        UpdateGalgameChoicesInput, UpdateGalgameLineInput, UpdateGalgameSceneInput,
        WebgalExportHostPort,
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
        input: a4n_orchestrator::galgame::GalgameAssetGenerationApplicationInput,
    ) -> Result<a4n_orchestrator::galgame::GalgameAssetGenerationApplicationSummary, AppError> {
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
    ) -> Result<galgame_line::Model, AppError> {
        self.orchestrator
            .insert_galgame_line_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn update_galgame_line_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameLineInput,
    ) -> Result<galgame_line::Model, AppError> {
        self.orchestrator
            .update_galgame_line_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn update_galgame_scene_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameSceneInput,
    ) -> Result<galgame_scene::Model, AppError> {
        self.orchestrator
            .update_galgame_scene_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn update_galgame_choices_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: UpdateGalgameChoicesInput,
    ) -> Result<Vec<galgame_choice::Model>, AppError> {
        self.orchestrator
            .update_galgame_choices_for_user_validated(user_id, project_id, input)
            .await
    }

    pub(crate) async fn reorder_galgame_lines_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: ReorderGalgameLinesInput,
    ) -> Result<Vec<galgame_line::Model>, AppError> {
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
        start_from_stage: Option<&str>,
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
        host_port: &dyn WebgalExportHostPort,
        user_id: i64,
        project_id: i64,
    ) -> Result<a4n_orchestrator::export::ExportFileData, AppError> {
        self.orchestrator
            .run_export_galgame_webgal_for_user_validated(host_port, user_id, project_id)
            .await
    }

    pub(crate) async fn bind_galgame_media_for_user_validated(
        &self,
        user_id: i64,
        input: BindGalgameMediaInput,
    ) -> Result<GalgameMediaBindingResult, AppError> {
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
        host_port: &dyn GalgameAudioHostPort,
        user_id: i64,
        input: ImportGalgameAudioInput,
    ) -> Result<GalgameMediaBindingResult, AppError> {
        self.orchestrator
            .import_galgame_audio_for_user_validated(host_port, user_id, input)
            .await
    }
}
