use super::*;

impl WorkerState {
    pub(super) async fn chapters(
        &self,
        request: WorkerEnvelope<SourceSnapshotProjectInput>,
    ) -> Result<Vec<SourceSnapshotChapterSummary>, WorkerFailure> {
        validate_request(&request)?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        let user_id = request.principal.require_user_id()?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        a4n_db::project::source_snapshot_chapters(self.database.connection(), project_id)
            .await
            .map_err(a4n_worker_db_adapter::map_project_mutation_error)
    }
}
