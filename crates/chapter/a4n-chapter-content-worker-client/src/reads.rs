use a4n_api_contract::chapter::{
    ChapterAnalysisEntry, ChapterContentReadApplicationPort, ChapterIndexEntry, ChapterSummary,
};
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_ANALYSES_OPERATION, CHAPTER_CONTENT_INDEX_OPERATION,
    CHAPTER_CONTENT_LIST_OPERATION, CHAPTER_CONTENT_SUMMARY_OPERATION, ChapterInput, ProjectInput,
};
use a4n_error::AppError;

use super::ChapterContentWorkerClient;

#[async_trait::async_trait]
impl ChapterContentReadApplicationPort for ChapterContentWorkerClient {
    async fn analyses(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterAnalysisEntry>, AppError> {
        let project_id = Self::positive("project_id", project_id)?;
        self.command(
            user_id,
            project_id,
            CHAPTER_CONTENT_ANALYSES_OPERATION,
            ProjectInput { project_id },
        )
        .await
    }

    async fn index(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ChapterIndexEntry>, AppError> {
        let project_id = Self::positive("project_id", project_id)?;
        self.command(
            user_id,
            project_id,
            CHAPTER_CONTENT_INDEX_OPERATION,
            ProjectInput { project_id },
        )
        .await
    }

    async fn list(&self, user_id: i64, project_id: i64) -> Result<Vec<ChapterSummary>, AppError> {
        let project_id = Self::positive("project_id", project_id)?;
        self.command(
            user_id,
            project_id,
            CHAPTER_CONTENT_LIST_OPERATION,
            ProjectInput { project_id },
        )
        .await
    }

    async fn summary(
        &self,
        user_id: i64,
        chapter_id: i64,
        project_id: Option<i64>,
    ) -> Result<ChapterSummary, AppError> {
        let chapter_id = Self::positive("chapter_id", chapter_id)?;
        self.command_with_project(
            user_id,
            project_id,
            CHAPTER_CONTENT_SUMMARY_OPERATION,
            ChapterInput { chapter_id },
        )
        .await
    }
}
