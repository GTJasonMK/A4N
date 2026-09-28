use super::SourceSnapshotWorkerClient;
use a4n_api_contract::project::{
    ProjectSourceSnapshotReadApplicationPort, SourceSnapshotChapterSummary,
};
use a4n_error::AppError;
use a4n_source_snapshot_worker_contract::SourceSnapshotProjectInput;

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
            SourceSnapshotProjectInput { project_id },
        )
        .await
    }
}
