use super::*;

impl WorkerState {
    pub(super) async fn invite_member(
        &self,
        request: WorkerEnvelope<InviteMemberInput>,
    ) -> Result<ProjectInviteSummary, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_INVITE_MEMBER_OPERATION,
            request.payload.project_id,
        )?;
        WorkerFailure::require_non_blank(
            &request.payload.username_or_email,
            "username_or_email 不能为空",
        )?;
        let user_id = request.principal.require_user_id()?;
        a4n_db::collaboration::invite_project_member(
            self.database.connection(),
            user_id,
            project_id,
            &request.payload.username_or_email,
            request.payload.role,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)
    }

    pub(super) async fn accept_invite(
        &self,
        request: WorkerEnvelope<InviteInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        unscoped_request(&request, PROJECT_COLLABORATION_ACCEPT_INVITE_OPERATION)?;
        WorkerFailure::require_positive_id(request.payload.invite_id, "invite_id")?;
        a4n_db::collaboration::accept_project_invite(
            self.database.connection(),
            request.principal.require_user_id()?,
            request.payload.invite_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(ProjectCollaborationAck {})
    }

    pub(super) async fn decline_invite(
        &self,
        request: WorkerEnvelope<InviteInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        unscoped_request(&request, PROJECT_COLLABORATION_DECLINE_INVITE_OPERATION)?;
        WorkerFailure::require_positive_id(request.payload.invite_id, "invite_id")?;
        a4n_db::collaboration::decline_project_invite(
            self.database.connection(),
            request.principal.require_user_id()?,
            request.payload.invite_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(ProjectCollaborationAck {})
    }

    pub(super) async fn revoke_invite(
        &self,
        request: WorkerEnvelope<ProjectInviteInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_REVOKE_INVITE_OPERATION,
            request.payload.project_id,
        )?;
        WorkerFailure::require_positive_id(request.payload.invite_id, "invite_id")?;
        a4n_db::collaboration::revoke_project_invite(
            self.database.connection(),
            request.principal.require_user_id()?,
            project_id,
            request.payload.invite_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(ProjectCollaborationAck {})
    }

    pub(super) async fn update_member_role(
        &self,
        request: WorkerEnvelope<UpdateMemberRoleInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_UPDATE_ROLE_OPERATION,
            request.payload.project_id,
        )?;
        WorkerFailure::require_positive_id(request.payload.member_user_id, "member_user_id")?;
        a4n_db::collaboration::update_project_member_role(
            self.database.connection(),
            request.principal.require_user_id()?,
            project_id,
            request.payload.member_user_id,
            request.payload.role,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(ProjectCollaborationAck {})
    }

    pub(super) async fn remove_member(
        &self,
        request: WorkerEnvelope<MemberInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_REMOVE_MEMBER_OPERATION,
            request.payload.project_id,
        )?;
        WorkerFailure::require_positive_id(request.payload.member_user_id, "member_user_id")?;
        a4n_db::collaboration::remove_project_member(
            self.database.connection(),
            request.principal.require_user_id()?,
            project_id,
            request.payload.member_user_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(ProjectCollaborationAck {})
    }

    pub(super) async fn leave_project(
        &self,
        request: WorkerEnvelope<ProjectInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_LEAVE_OPERATION,
            request.payload.project_id,
        )?;
        a4n_db::collaboration::leave_project(
            self.database.connection(),
            request.principal.require_user_id()?,
            project_id,
        )
        .await
        .map_err(a4n_worker_db_adapter::map_collaboration_db_error)?;
        Ok(ProjectCollaborationAck {})
    }
}
