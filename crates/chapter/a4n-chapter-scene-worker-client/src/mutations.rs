use super::*;

#[async_trait::async_trait]
impl ChapterSceneMutationApplicationPort for ChapterSceneWorkerClient {
    async fn update(
        &self,
        user_id: i64,
        input: UpdateChapterSceneInput,
    ) -> Result<ChapterSceneSummary, AppError> {
        self.command(
            user_id,
            CHAPTER_SCENE_UPDATE_OPERATION,
            UpdateSceneInput {
                scene_id: a4n_worker_client::positive_id("scene_id", input.scene_id)?,
                content: input.content,
                title: input.title,
                summary: input.summary,
                scene_type: input.scene_type,
                location: input.location,
                timeline_position: input.timeline_position,
            },
        )
        .await
    }

    async fn delete(&self, user_id: i64, scene_id: i64) -> Result<(), AppError> {
        let _: serde_json::Value = self
            .command(
                user_id,
                CHAPTER_SCENE_DELETE_OPERATION,
                DeleteSceneInput {
                    scene_id: a4n_worker_client::positive_id("scene_id", scene_id)?,
                },
            )
            .await?;
        Ok(())
    }

    async fn reorder(
        &self,
        user_id: i64,
        input: ReorderChapterScenesInput,
    ) -> Result<Vec<ChapterSceneSummary>, AppError> {
        if input.order.is_empty() {
            return Err(AppError::InvalidArgument("order 不能为空".into()));
        }
        self.command(
            user_id,
            CHAPTER_SCENE_REORDER_OPERATION,
            ReorderScenesInput { order: input.order },
        )
        .await
    }
}
