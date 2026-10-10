use super::*;

impl WorkerState {
    pub(super) async fn verify(
        &self,
        request: WorkerEnvelope<ProjectAccessInput>,
    ) -> Result<ProjectAccessRole, WorkerFailure> {
        validate_user_request(&request, PROJECT_ACCESS_VERIFY_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        let project_id = request.require_project_scope(request.payload.project_id)?;
        a4n_db::collaboration::verify_project_permission(
            self.database.connection(),
            project_id,
            user_id,
            request.payload.required,
        )
        .await
        .map_err(collaboration_error)
    }

    pub(super) async fn verify_batch(
        &self,
        request: WorkerEnvelope<ProjectAccessBatchInput>,
    ) -> Result<(), WorkerFailure> {
        validate_user_request(&request, PROJECT_ACCESS_VERIFY_BATCH_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        a4n_db::collaboration::verify_project_permissions(
            self.database.connection(),
            &request.payload.project_ids,
            user_id,
            request.payload.required,
        )
        .await
        .map_err(collaboration_error)
    }

    pub(super) async fn filter(
        &self,
        request: WorkerEnvelope<ProjectAccessBatchInput>,
    ) -> Result<Vec<i64>, WorkerFailure> {
        validate_user_request(&request, PROJECT_ACCESS_FILTER_OPERATION)?;
        let user_id = request.principal.parse_user_id()?;
        a4n_db::collaboration::project_ids_with_permission(
            self.database.connection(),
            &request.payload.project_ids,
            user_id,
            request.payload.required,
        )
        .await
        .map_err(collaboration_error)
    }

    pub(super) async fn root(
        &self,
        request: WorkerEnvelope<ProjectAccessRootInput>,
    ) -> Result<ProjectCollaborationRootSummary, WorkerFailure> {
        request_validate(&request, PROJECT_ACCESS_ROOT_OPERATION)?;
        if request.principal.kind != WorkerPrincipalKind::Service
            || request.principal.subject != "runtime"
        {
            return Err(WorkerFailure::unauthorized(
                "project access root 仅允许 runtime service principal",
            ));
        }
        let project_id = request.require_project_scope(request.payload.project_id)?;
        let root =
            a4n_db::collaboration::resolve_root_project(self.database.connection(), project_id)
                .await
                .map_err(collaboration_error)?;
        Ok(ProjectCollaborationRootSummary {
            id: root.id,
            user_id: root.user_id,
        })
    }
}
