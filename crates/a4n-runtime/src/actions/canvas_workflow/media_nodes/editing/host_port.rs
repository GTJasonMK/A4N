use std::future::Future;

use a4n_core::error::AppError;
use a4n_orchestrator::canvas::{
    CanvasAssembleVideosHostInput, CanvasEditingNodeHostPort, CanvasExtractFrameHostInput,
    CanvasGeneratedImageArtifact, CanvasGeneratedVideoArtifact, CanvasTrimVideoHostInput,
};
use a4n_queue::{
    priority::{TaskKind as QueueTaskKind, TaskPriority},
    queue::TaskWorkflowContext,
};
use async_trait::async_trait;

use crate::{
    actions::video_records::{assemble_canvas_videos, extract_canvas_video_frame, trim_video},
    state::AppState,
};

pub(super) struct RuntimeCanvasEditingNodeHostPort<'a> {
    state: &'a AppState,
}

impl<'a> RuntimeCanvasEditingNodeHostPort<'a> {
    pub(super) fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    async fn run_tracked<F, Fut, T>(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, String>>,
    {
        let workflow_context = TaskWorkflowContext::new(workflow_run_id)
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))?;
        self.state
            .task_queue()
            .queue_run_tracked_with_workflow_context(
                QueueTaskKind::Assembly,
                TaskPriority::Medium,
                Some(project_id),
                workflow_context,
                || async {
                    operation()
                        .await
                        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
                },
            )
            .await
    }
}

#[async_trait]
impl CanvasEditingNodeHostPort for RuntimeCanvasEditingNodeHostPort<'_> {
    async fn extract_frame(
        &self,
        input: CanvasExtractFrameHostInput,
    ) -> Result<CanvasGeneratedImageArtifact, AppError> {
        let image = self
            .run_tracked(input.project_id, input.workflow_run_id, || async {
                extract_canvas_video_frame(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.video_id,
                    input.frame_kind,
                    input.at_secs,
                )
                .await
            })
            .await?;
        Ok(CanvasGeneratedImageArtifact {
            generated_image_id: image.id,
        })
    }

    async fn trim_video(
        &self,
        input: CanvasTrimVideoHostInput,
    ) -> Result<CanvasGeneratedVideoArtifact, AppError> {
        let video = self
            .run_tracked(input.project_id, input.workflow_run_id, || async {
                trim_video(
                    self.state,
                    input.user_id,
                    input.video_id,
                    input.start_secs,
                    input.end_secs,
                )
                .await
            })
            .await?;
        Ok(CanvasGeneratedVideoArtifact {
            generated_video_id: video.id,
        })
    }

    async fn assemble_videos(
        &self,
        input: CanvasAssembleVideosHostInput,
    ) -> Result<CanvasGeneratedVideoArtifact, AppError> {
        let video = self
            .run_tracked(input.project_id, input.workflow_run_id, || async {
                assemble_canvas_videos(
                    self.state,
                    input.user_id,
                    input.project_id,
                    input.video_ids,
                    input.title,
                )
                .await
            })
            .await?;
        Ok(CanvasGeneratedVideoArtifact {
            generated_video_id: video.id,
        })
    }
}
