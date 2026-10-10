use super::*;

impl WorkerState {
    pub(super) async fn list_members(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ProjectMemberSummary>, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION,
            request.payload.project_id,
        )?;
        let user_id = request.principal.require_user_id()?;
        a4n_db::collaboration::list_project_members(self.database.connection(), user_id, project_id)
            .await
            .map_err(collaboration_error)
    }

    pub(super) async fn list_invites(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<Vec<ProjectInviteSummary>, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_LIST_INVITES_OPERATION,
            request.payload.project_id,
        )?;
        let user_id = request.principal.require_user_id()?;
        a4n_db::collaboration::list_project_invites(self.database.connection(), user_id, project_id)
            .await
            .map_err(collaboration_error)
    }

    pub(super) async fn list_my_invites(
        &self,
        request: WorkerEnvelope<EmptyInput>,
    ) -> Result<Vec<ProjectInviteSummary>, WorkerFailure> {
        unscoped_request(&request, PROJECT_COLLABORATION_LIST_MY_INVITES_OPERATION)?;
        a4n_db::collaboration::list_my_project_invites(
            self.database.connection(),
            request.principal.require_user_id()?,
        )
        .await
        .map_err(collaboration_error)
    }
}
