//! 视频 trim port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoTrimApplicationPort for crate::OrchestratorContext {
    async fn prepare_trim(
        &self,
        user_id: i64,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, AppError> {
        let source = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            source.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_video_trim_validated(source, start_secs, end_secs)
    }

    async fn create_trimmed_record(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.create_trimmed_video_record_validated(plan).await
    }
}
