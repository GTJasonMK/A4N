use super::*;

#[async_trait::async_trait]
impl OutlineMutationApplicationPort for OutlineWorkerClient {
    async fn delete_latest_chapters(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.command(user_id, input.project_id, DELETE_CHAPTERS_OPERATION, input)
            .await
    }

    async fn delete_latest_parts(
        &self,
        user_id: i64,
        input: DeleteLatestOutlineRecordsInput,
    ) -> Result<OutlineMutationResult, AppError> {
        self.command(user_id, input.project_id, DELETE_PARTS_OPERATION, input)
            .await
    }

    async fn save_chapter(
        &self,
        user_id: i64,
        input: SaveChapterOutlineInput,
    ) -> Result<SaveChapterOutlineResult, AppError> {
        self.command(user_id, input.project_id, SAVE_CHAPTER_OPERATION, input)
            .await
    }

    async fn update_chapter(
        &self,
        user_id: i64,
        input: UpdateChapterOutlineInput,
    ) -> Result<UpdateChapterOutlineResult, AppError> {
        self.command(user_id, input.project_id, UPDATE_CHAPTER_OPERATION, input)
            .await
    }

    async fn save_part(
        &self,
        user_id: i64,
        input: SavePartOutlineInput,
    ) -> Result<SavePartOutlineResult, AppError> {
        self.command(user_id, input.project_id, SAVE_PART_OPERATION, input)
            .await
    }
}
