//! 视频失败与生命周期 ports。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoRecordFailureApplicationPort for crate::OrchestratorContext {
    async fn mark_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<GeneratedVideoSummary, AppError> {
        Ok(generated_video_summary(
            &self
                .mark_generated_video_error_status_and_prompt(video_id, prompt_id, status, message)
                .await?,
        ))
    }
}

#[async_trait]
impl VideoRecordLifecycleApplicationPort for crate::OrchestratorContext {
    async fn delete_record_with_cleanup(&self, video_id: i64) -> Result<(), AppError> {
        self.delete_generated_video_record_with_cleanup(video_id)
            .await
    }
}
