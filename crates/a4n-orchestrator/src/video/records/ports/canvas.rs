//! 画布媒体 port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoCanvasMediaApplicationPort for crate::OrchestratorContext {
    async fn prepare_canvas_assembly(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, AppError> {
        self.prepare_canvas_assembly_validated(project_id, video_ids, title)
            .await
    }

    async fn create_canvas_assembly_record(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.create_canvas_assembly_record_validated(project_id, plan)
            .await
    }

    async fn prepare_canvas_frame(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: CanvasFrameKind,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, AppError> {
        self.prepare_canvas_frame_validated(project_id, video_id, frame_kind, at_secs)
            .await
    }

    async fn create_canvas_frame_record(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<GeneratedImageSummary, AppError> {
        self.create_canvas_frame_record_validated(plan, file_path, file_size)
            .await
    }
}
