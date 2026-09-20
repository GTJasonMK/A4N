//! 视频读取 port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoRecordReadApplicationPort for crate::OrchestratorContext {
    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, AppError> {
        let video = self.required_video_record(video_id).await?;
        self.verify_project_permission(
            video.project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(generated_video_summary(&video))
    }

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Viewer,
        )
        .await?;
        Ok(self
            .list_video_records_by_chapter_validated(project_id, chapter_id)
            .await?
            .iter()
            .map(generated_video_summary)
            .collect())
    }

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        let images = self
            .validated_video_reference_images(project_id, chapter_id, reference_image_ids_json)
            .await?;
        let project_ids = images
            .iter()
            .map(|image| image.project_id)
            .collect::<HashSet<_>>();
        self.verify_project_permissions(
            &project_ids.into_iter().collect::<Vec<_>>(),
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(images
            .iter()
            .map(crate::media::generated_image_summary)
            .collect())
    }
}
