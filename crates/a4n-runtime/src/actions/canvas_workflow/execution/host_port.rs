use a4n_orchestrator::canvas::{
    CanvasGraphExecutionHostPort, CanvasGraphNodeExecutionInput, CanvasGraphNodeExecutionKind,
    CanvasNodeRunSummary,
};
use async_trait::async_trait;

use crate::{
    actions::{
        access,
        canvas_workflow::{
            CanvasRunSummary,
            media_nodes::{
                run_canvas_editing_node, run_canvas_generate_image_node,
                run_canvas_generate_video_node,
            },
            node_lifecycle::CanvasRunHandle,
            output_nodes::run_canvas_output_node,
            probe_node::run_canvas_probe_node,
            text_nodes::{run_canvas_llm_text_node, run_canvas_render_text_node},
        },
    },
    state::AppState,
};

pub(super) struct RuntimeCanvasGraphExecutionHostPort<'a> {
    state: &'a AppState,
}

impl<'a> RuntimeCanvasGraphExecutionHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self { state }
    }
}

#[async_trait]
impl CanvasGraphExecutionHostPort for RuntimeCanvasGraphExecutionHostPort<'_> {
    async fn verify_project_editor(&self, user_id: i64, project_id: i64) -> Result<(), String> {
        access::verify_project_editor_string(self.state, user_id, project_id).await
    }

    async fn execute_node(
        &self,
        input: CanvasGraphNodeExecutionInput<'_>,
    ) -> Result<CanvasNodeRunSummary, String> {
        let run_handle = Some(CanvasRunHandle {
            workflow_run_id: input.shared_run.workflow_run_id,
            finalize_on_success: input.shared_run.finalize_on_success,
        });
        let summary = match input.kind {
            CanvasGraphNodeExecutionKind::RenderText => {
                run_canvas_render_text_node(
                    self.state,
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
            CanvasGraphNodeExecutionKind::LlmText => {
                run_canvas_llm_text_node(
                    self.state,
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
            CanvasGraphNodeExecutionKind::GenerateImage => {
                run_canvas_generate_image_node(
                    self.state,
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
            CanvasGraphNodeExecutionKind::GenerateVideo => {
                run_canvas_generate_video_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
            CanvasGraphNodeExecutionKind::ExtractFrame
            | CanvasGraphNodeExecutionKind::TrimVideo
            | CanvasGraphNodeExecutionKind::AssembleVideo => {
                run_canvas_editing_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
            CanvasGraphNodeExecutionKind::Probe => {
                run_canvas_probe_node(
                    &self.state.canvas(),
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
            CanvasGraphNodeExecutionKind::TextOutput
            | CanvasGraphNodeExecutionKind::ImageOutput
            | CanvasGraphNodeExecutionKind::VideoOutput
            | CanvasGraphNodeExecutionKind::AudioOutput => {
                run_canvas_output_node(
                    &self.state.canvas(),
                    input.project_id,
                    input.graph_id,
                    input.node,
                    input.document,
                    run_handle,
                    input.mode,
                )
                .await
            }
        }?;
        Ok(canvas_node_run_summary(summary))
    }
}

fn canvas_node_run_summary(summary: CanvasRunSummary) -> CanvasNodeRunSummary {
    CanvasNodeRunSummary {
        workflow_run_id: summary.workflow_run_id,
        project_id: summary.project_id,
        graph_id: summary.graph_id,
        mode: summary.mode,
        status: summary.status,
        error_message: summary.error_message,
    }
}
