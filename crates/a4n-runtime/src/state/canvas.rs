//! 画布领域 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::{canvas::canvas_text_asset, media::media_asset};
use a4n_orchestrator::{
    OrchestratorContext,
    canvas::{
        CanvasAudioImportHostInput, CanvasEditingNodeExecutionInput,
        CanvasEditingNodeHostPort, CanvasGraphExecutionHostPort, CanvasGraphRecordDocument,
        CanvasGraphRecordInput, CanvasGraphResumePreparationInput, CanvasGraphRunPreparationInput,
        CanvasImageImportApplicationResult, CanvasImageImportHostInput,
        CanvasImageNodeExecutionInput, CanvasLlmTextInput, CanvasNodeRunSummary,
        CanvasOutputNodeExecutionInput, CanvasProbeNodeExecutionInput,
        CanvasSingleNodeRunPreparationInput, CanvasTextAssetRecordInput,
        CanvasTextGenerationLlmPort, CanvasTextNodeExecutionInput, CanvasTextNodeRunSummary,
        CanvasVideoGenerationHostPort, CanvasVideoImportApplicationResult,
        CanvasVideoImportHostInput, CanvasVideoImportHostPort, CanvasVideoNodeExecutionInput,
        CanvasVideoRequestPreview, GenerateCanvasVideoInput, PreparedCanvasGraphRun,
    },
};
use chrono::NaiveDateTime;

pub(crate) struct CanvasRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> CanvasRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn get_or_create_canvas_graph_record(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        self.orchestrator
            .get_or_create_canvas_graph_record(project_id, graph_id, now)
            .await
    }

    pub(crate) async fn prepare_canvas_llm_text_input_validated(
        &self,
        workflow_run_id: Option<i64>,
        node: &a4n_core::models::canvas::canvas_node::Model,
        document: &CanvasGraphRecordDocument,
    ) -> Result<CanvasLlmTextInput, AppError> {
        self.orchestrator
            .prepare_canvas_llm_text_input_validated(workflow_run_id, node, document)
            .await
    }

    pub(crate) async fn run_canvas_render_text_node_validated(
        &self,
        input: CanvasTextNodeExecutionInput<'_>,
    ) -> Result<CanvasTextNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_render_text_node_validated(input)
            .await
    }

    pub(crate) async fn run_canvas_llm_text_node_validated(
        &self,
        input: CanvasTextNodeExecutionInput<'_>,
        llm_port: &dyn CanvasTextGenerationLlmPort,
    ) -> Result<CanvasTextNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_llm_text_node_validated(input, llm_port)
            .await
    }

    pub(crate) async fn run_canvas_image_node_validated(
        &self,
        input: CanvasImageNodeExecutionInput<'_>,
        host_port: &dyn a4n_orchestrator::media::ImageGenerationHostPort,
        progress: &a4n_orchestrator::media::ImageGenerationProgress<'_>,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_image_node_validated(input, host_port, progress)
            .await
    }

    pub(crate) async fn run_canvas_editing_node_validated(
        &self,
        input: CanvasEditingNodeExecutionInput<'_>,
        host_port: &dyn CanvasEditingNodeHostPort,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_editing_node_validated(input, host_port)
            .await
    }

    pub(crate) async fn run_canvas_output_node_validated(
        &self,
        input: CanvasOutputNodeExecutionInput<'_>,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_output_node_validated(input)
            .await
    }

    pub(crate) async fn run_canvas_probe_node_validated(
        &self,
        input: CanvasProbeNodeExecutionInput<'_>,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_probe_node_validated(input)
            .await
    }

    pub(crate) async fn prepare_canvas_graph_run_validated(
        &self,
        input: CanvasGraphRunPreparationInput,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        self.orchestrator
            .prepare_canvas_graph_run_validated(input, host_port)
            .await
    }

    pub(crate) async fn prepare_canvas_single_node_run_validated(
        &self,
        input: CanvasSingleNodeRunPreparationInput,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        self.orchestrator
            .prepare_canvas_single_node_run_validated(input, host_port)
            .await
    }

    pub(crate) async fn prepare_canvas_graph_resume_validated(
        &self,
        input: CanvasGraphResumePreparationInput,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<PreparedCanvasGraphRun, String> {
        self.orchestrator
            .prepare_canvas_graph_resume_validated(input, host_port)
            .await
    }

    pub(crate) async fn run_prepared_canvas_graph_validated(
        &self,
        prepared: PreparedCanvasGraphRun,
        host_port: &dyn CanvasGraphExecutionHostPort,
    ) -> Result<CanvasNodeRunSummary, String> {
        self.orchestrator
            .run_prepared_canvas_graph_validated(prepared, host_port)
            .await
    }

    pub(crate) async fn save_canvas_graph_record(
        &self,
        project_id: i64,
        input: CanvasGraphRecordInput,
        now: NaiveDateTime,
    ) -> Result<CanvasGraphRecordDocument, AppError> {
        self.orchestrator
            .save_canvas_graph_record(project_id, input, now)
            .await
    }

    pub(crate) async fn upsert_canvas_text_asset_record(
        &self,
        project_id: i64,
        input: CanvasTextAssetRecordInput,
        now: NaiveDateTime,
    ) -> Result<canvas_text_asset::Model, AppError> {
        self.orchestrator
            .upsert_canvas_text_asset_record(project_id, input, now)
            .await
    }

    pub(crate) async fn import_canvas_audio_validated(
        &self,
        input: CanvasAudioImportHostInput,
    ) -> Result<media_asset::Model, AppError> {
        self.orchestrator
            .import_canvas_audio_validated(input)
            .await
    }

    pub(crate) async fn import_canvas_image_validated(
        &self,
        input: CanvasImageImportHostInput,
    ) -> Result<CanvasImageImportApplicationResult, AppError> {
        self.orchestrator
            .import_canvas_image_validated(input)
            .await
    }

    pub(crate) async fn import_canvas_video_validated(
        &self,
        host_port: &dyn CanvasVideoImportHostPort,
        input: CanvasVideoImportHostInput,
    ) -> Result<CanvasVideoImportApplicationResult, AppError> {
        self.orchestrator
            .import_canvas_video_validated(host_port, input)
            .await
    }

    pub(crate) async fn run_canvas_video_node_validated(
        &self,
        input: CanvasVideoNodeExecutionInput<'_>,
        host_port: &dyn CanvasVideoGenerationHostPort,
    ) -> Result<CanvasNodeRunSummary, AppError> {
        self.orchestrator
            .run_canvas_video_node_validated(input, host_port)
            .await
    }

    pub(crate) async fn preview_canvas_video_request_validated(
        &self,
        input: GenerateCanvasVideoInput,
        host_port: &dyn CanvasVideoGenerationHostPort,
    ) -> Result<CanvasVideoRequestPreview, AppError> {
        self.orchestrator
            .preview_canvas_video_request_validated(input, host_port)
            .await
    }
}
