use super::*;

#[async_trait::async_trait]
impl MangaRecordReadApplicationPort for MangaRecordWorkerClient {
    async fn list_page_prompts(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPagePromptSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            MANGA_RECORD_LIST_OPERATION,
            MangaChapterInput {
                project_id,
                chapter_id,
            },
        )
        .await
    }
}
