use super::*;

#[async_trait::async_trait]
impl MangaPipelineReadApplicationPort for MangaRecordWorkerClient {
    async fn status(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<MangaPipelineStatusSummary, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_PIPELINE_STATUS_OPERATION,
            MangaChapterInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }

    async fn statuses(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: Vec<i64>,
    ) -> Result<Vec<MangaPipelineStatusSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_PIPELINE_STATUSES_OPERATION,
            MangaPipelineStatusesInput {
                project_id,
                chapter_ids,
            },
        )
        .await
    }
}
