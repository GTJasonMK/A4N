use super::*;

#[async_trait::async_trait]
impl ChapterSceneReadApplicationPort for ChapterSceneWorkerClient {
    async fn list(
        &self,
        user_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterSceneSummary>, AppError> {
        self.command(
            user_id,
            CHAPTER_SCENE_LIST_OPERATION,
            ChapterInput {
                chapter_id: a4n_worker_client::positive_id("chapter_id", chapter_id)?,
            },
        )
        .await
    }
}
