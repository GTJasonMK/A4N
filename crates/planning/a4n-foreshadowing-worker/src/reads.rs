use super::*;

impl WorkerState {
    pub(super) async fn list(
        &self,
        request: WorkerEnvelope<ForeshadowingProjectInput>,
    ) -> Result<Vec<ForeshadowingSummary>, WorkerFailure> {
        validate_request(&request, FORESHADOWING_LIST_OPERATION)?;
        let project_id = request.payload.project_id;
        if request.project_id != Some(project_id) || project_id <= 0 {
            return Err(WorkerFailure::invalid_request(
                "worker scope project_id 与 payload 不一致或无效",
            ));
        }
        let user_id = user_id(&request)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_error)?;
        a4n_db::project::foreshadowing_index_records_by_project(
            self.database.connection(),
            project_id,
        )
        .await
        .map_err(foreshadowing_error)?
        .into_iter()
        .map(foreshadowing_summary)
        .collect()
    }
}
