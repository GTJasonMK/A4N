use a4n_api_contract::chapter::{ImportChapterInput, UpdateChapterContentInput};
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_IMPORT_OPERATION, CHAPTER_CONTENT_UPDATE_OPERATION,
    ChapterContentMutationResult,
};
use a4n_error::AppError;

use super::ChapterContentWorkerClient;

impl ChapterContentWorkerClient {
    pub async fn update_content(
        &self,
        user_id: i64,
        input: UpdateChapterContentInput,
    ) -> Result<ChapterContentMutationResult, AppError> {
        Self::positive("chapter_id", input.chapter_id)?;
        self.command_global(user_id, CHAPTER_CONTENT_UPDATE_OPERATION, input)
            .await
    }

    pub async fn import_content(
        &self,
        user_id: i64,
        input: ImportChapterInput,
    ) -> Result<ChapterContentMutationResult, AppError> {
        Self::positive("project_id", input.project_id)?;
        self.command(
            user_id,
            input.project_id,
            CHAPTER_CONTENT_IMPORT_OPERATION,
            input,
        )
        .await
    }
}
