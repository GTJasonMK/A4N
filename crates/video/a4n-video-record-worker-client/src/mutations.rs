use super::*;

impl VideoRecordWorkerClient {
    pub async fn update_record(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.command(
            user_id,
            None,
            VIDEO_RECORD_UPDATE_OPERATION,
            VideoUpdateInput { video_id, input },
        )
        .await
    }

    pub async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, AppError> {
        self.command(
            user_id,
            Some(project_id),
            VIDEO_RECORD_SELECT_SEGMENT_OPERATION,
            VideoSegmentSelectionInput {
                project_id,
                chapter_id,
                video_id,
                input,
            },
        )
        .await
    }
}
