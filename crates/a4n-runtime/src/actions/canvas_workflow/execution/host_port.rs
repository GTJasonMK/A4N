use a4n_api_contract::canvas::CanvasRunSummary;
use a4n_canvas_contract::{
    CanvasEditingNodeRun, CanvasGraphExecutionHostPort, CanvasGraphNodeExecutionInput,
    CanvasGraphNodeExecutionKind, CanvasNodeRun,
};
use async_trait::async_trait;

use crate::{
    actions::{
        access,
        canvas_workflow::{
            media_nodes::{
                run_canvas_editing_node, run_canvas_generate_image_node,
                run_canvas_generate_video_node,
            },
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
        input: CanvasGraphNodeExecutionInput,
    ) -> Result<CanvasRunSummary, String> {
        let summary = match input.kind {
            CanvasGraphNodeExecutionKind::RenderText => {
                run_canvas_render_text_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    Some(CanvasNodeRun {
                        workflow_run_id: input.shared_run.workflow_run_id,
                        finalize_on_success: input.shared_run.finalize_on_success,
                    }),
                    input.mode.as_str(),
                )
                .await
            }
            CanvasGraphNodeExecutionKind::LlmText => {
                run_canvas_llm_text_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    Some(CanvasNodeRun {
                        workflow_run_id: input.shared_run.workflow_run_id,
                        finalize_on_success: input.shared_run.finalize_on_success,
                    }),
                    input.mode.as_str(),
                )
                .await
            }
            CanvasGraphNodeExecutionKind::GenerateImage => {
                run_canvas_generate_image_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    Some(CanvasNodeRun {
                        workflow_run_id: input.shared_run.workflow_run_id,
                        finalize_on_success: input.shared_run.finalize_on_success,
                    }),
                    input.mode.as_str(),
                )
                .await
            }
            CanvasGraphNodeExecutionKind::GenerateVideo => {
                run_canvas_generate_video_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    Some(CanvasNodeRun {
                        workflow_run_id: input.shared_run.workflow_run_id,
                        finalize_on_success: input.shared_run.finalize_on_success,
                    }),
                    input.mode.as_str(),
                )
                .await
            }
            CanvasGraphNodeExecutionKind::ExtractFrame
            | CanvasGraphNodeExecutionKind::TrimVideo
            | CanvasGraphNodeExecutionKind::AssembleVideo => {
                let editing_run = Some(CanvasEditingNodeRun {
                    workflow_run_id: input.shared_run.workflow_run_id,
                    finalize_on_success: input.shared_run.finalize_on_success,
                });
                run_canvas_editing_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    editing_run,
                    input.mode.as_str(),
                )
                .await
            }
            CanvasGraphNodeExecutionKind::Probe => {
                run_canvas_probe_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    Some(CanvasNodeRun {
                        workflow_run_id: input.shared_run.workflow_run_id,
                        finalize_on_success: input.shared_run.finalize_on_success,
                    }),
                    input.mode.as_str(),
                )
                .await
            }
            CanvasGraphNodeExecutionKind::TextOutput
            | CanvasGraphNodeExecutionKind::ImageOutput
            | CanvasGraphNodeExecutionKind::VideoOutput
            | CanvasGraphNodeExecutionKind::AudioOutput => {
                run_canvas_output_node(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.graph_id,
                    input.node_id,
                    Some(CanvasNodeRun {
                        workflow_run_id: input.shared_run.workflow_run_id,
                        finalize_on_success: input.shared_run.finalize_on_success,
                    }),
                    input.mode.as_str(),
                )
                .await
            }
        }?;
        Ok(summary)
    }
}
