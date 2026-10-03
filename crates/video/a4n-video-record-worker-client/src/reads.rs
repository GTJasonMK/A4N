use super::*;

#[async_trait::async_trait]
impl VideoRecordReadApplicationPort for VideoRecordWorkerClient {
    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.command(
            user_id,
            None,
            VIDEO_RECORD_PLAYBACK_OPERATION,
            VideoIdInput { video_id },
        )
        .await
    }

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_RECORD_LIST_OPERATION,
            ChapterVideosInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: Option<i64>,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<GeneratedImageSummary>, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_RECORD_REFERENCE_IMAGES_OPERATION,
            ReferenceImagesInput {
                project_id,
                chapter_id,
                reference_image_ids_json: reference_image_ids_json.cloned(),
            },
        )
        .await
    }
}
