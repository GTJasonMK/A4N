use super::SourceSnapshotWorkerClient;
use a4n_api_contract::project::{
    ProjectSourceSnapshotReadApplicationPort, SourceSnapshotChapterIndexEntry,
    SourceSnapshotChapterSummary,
};
use a4n_error::AppError;
use a4n_source_snapshot_worker_contract::{
    SOURCE_SNAPSHOT_CHAPTER_INDEX_OPERATION, SOURCE_SNAPSHOT_CHAPTER_OPERATION,
    SOURCE_SNAPSHOT_CHAPTERS_OPERATION, SourceSnapshotChapterInput, SourceSnapshotProjectInput,
};

#[async_trait::async_trait]
impl ProjectSourceSnapshotReadApplicationPort for SourceSnapshotWorkerClient {
    async fn chapters(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, AppError> {
        self.command(
            user_id,
            project_id,
            SOURCE_SNAPSHOT_CHAPTERS_OPERATION,
            SourceSnapshotProjectInput { project_id },
        )
        .await
    }

    async fn chapter_index(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<SourceSnapshotChapterIndexEntry>, AppError> {
        self.command(
            user_id,
            project_id,
            SOURCE_SNAPSHOT_CHAPTER_INDEX_OPERATION,
            SourceSnapshotProjectInput { project_id },
        )
        .await
    }

    async fn chapter(
        &self,
        user_id: i64,
        project_id: i64,
        snapshot_id: i64,
        source_chapter_id: i64,
    ) -> Result<SourceSnapshotChapterSummary, AppError> {
        self.command(
            user_id,
            project_id,
            SOURCE_SNAPSHOT_CHAPTER_OPERATION,
            SourceSnapshotChapterInput {
                project_id,
                snapshot_id,
                source_chapter_id,
            },
        )
        .await
    }
}
