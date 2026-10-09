use super::*;

impl WorkerState {
    async fn verify_project_read<T: serde::Serialize>(
        &self,
        request: &WorkerEnvelope<T>,
        project_id: i64,
        operation_id: &str,
    ) -> Result<(), WorkerFailure> {
        validate_request(request, operation_id)?;
        if request.project_id != Some(project_id) || project_id <= 0 {
            return Err(WorkerFailure::invalid_request(
                "worker scope project_id 与 payload 不一致或无效",
            ));
        }
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id(request)?,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map(|_| ())
        .map_err(collaboration_error)
    }

    pub(super) async fn chapters(
        &self,
        request: WorkerEnvelope<SourceSnapshotProjectInput>,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, WorkerFailure> {
        let project_id = request.payload.project_id;
        self.verify_project_read(&request, project_id, SOURCE_SNAPSHOT_CHAPTERS_OPERATION)
            .await?;
        a4n_db::project::source_snapshot_chapters(self.database.connection(), project_id)
            .await
            .map_err(project_error)
    }

    pub(super) async fn chapter_index(
        &self,
        request: WorkerEnvelope<SourceSnapshotProjectInput>,
    ) -> Result<Vec<SourceSnapshotChapterIndexEntry>, WorkerFailure> {
        let project_id = request.payload.project_id;
        self.verify_project_read(
            &request,
            project_id,
            SOURCE_SNAPSHOT_CHAPTER_INDEX_OPERATION,
        )
        .await?;
        a4n_db::project::source_snapshot_chapter_index(self.database.connection(), project_id)
            .await
            .map_err(project_error)
    }

    pub(super) async fn chapter(
        &self,
        request: WorkerEnvelope<SourceSnapshotChapterInput>,
    ) -> Result<SourceSnapshotChapterSummary, WorkerFailure> {
        let project_id = request.payload.project_id;
        self.verify_project_read(&request, project_id, SOURCE_SNAPSHOT_CHAPTER_OPERATION)
            .await?;
        a4n_db::project::source_snapshot_chapter_for_project(
            self.database.connection(),
            project_id,
            request.payload.snapshot_id,
            request.payload.source_chapter_id,
        )
        .await
        .map_err(project_error)?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!(
                "源快照章节不存在: project_id={project_id}, source_chapter_id={}",
                request.payload.source_chapter_id
            ))
        })
    }
}
