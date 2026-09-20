//! 视频修改 port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoRecordMutationApplicationPort for crate::OrchestratorContext {
    async fn update(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let existing = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            existing.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(
            &self.update_video_record_validated(video_id, input).await?,
        ))
    }

    async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(
            &self
                .select_segment_video_validated(
                    project_id,
                    chapter_id,
                    input.segment_plan_prompt_id,
                    (
                        &input.target_segment.source_shot_id,
                        input.target_segment.segment_index_in_shot,
                    ),
                    video_id,
                )
                .await?,
        ))
    }

    async fn delete(&self, user_id: i64, video_id: i64) -> Result<(), AppError> {
        let project_ids = self.missing_video_reference_project_ids(video_id).await?;
        if project_ids.is_empty() {
            return Err(AppError::NotFound("视频不存在".to_string()));
        }
        self.verify_project_permissions(
            &project_ids,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.delete_video_record_validated(video_id).await
    }
}
