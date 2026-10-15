use super::*;

impl WorkerState {
    async fn verify_project_read<T: serde::Serialize>(
        &self,
        request: &WorkerEnvelope<T>,
        operation_id: &str,
        requested_project_id: i64,
    ) -> Result<i64, WorkerFailure> {
        super::validate_request(request, operation_id)?;
        let project_id = request.require_project_scope(requested_project_id)?;
        let user_id = request.principal.require_user_id()?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(project_id)
    }

    pub(super) async fn chapters(
        &self,
        request: WorkerEnvelope<SourceSnapshotProjectInput>,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, WorkerFailure> {
        let project_id = self
            .verify_project_read(
                &request,
                SOURCE_SNAPSHOT_CHAPTERS_OPERATION,
                request.payload.project_id,
            )
            .await?;
        a4n_db::project::source_snapshot_chapters(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_project_mutation_error)
    }

    pub(super) async fn chapter_index(
        &self,
        request: WorkerEnvelope<SourceSnapshotProjectInput>,
    ) -> Result<Vec<SourceSnapshotChapterIndex>, WorkerFailure> {
        let project_id = self
            .verify_project_read(
                &request,
                SOURCE_SNAPSHOT_CHAPTER_INDEX_OPERATION,
                request.payload.project_id,
            )
            .await?;
        a4n_db::project::source_snapshot_chapter_index(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_project_mutation_error)
    }

    pub(super) async fn chapter(
        &self,
        request: WorkerEnvelope<SourceSnapshotChapterInput>,
    ) -> Result<SourceSnapshotChapterSummary, WorkerFailure> {
        let project_id = self
            .verify_project_read(
                &request,
                SOURCE_SNAPSHOT_CHAPTER_OPERATION,
                request.payload.project_id,
            )
            .await?;
        a4n_db::project::source_snapshot_chapter_for_project(
            self.database.connection(),
            project_id,
            request.payload.source_chapter_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_project_mutation_error)
    }
}
