use super::*;

impl WorkerState {
    pub(super) async fn invite_member(
        &self,
        request: WorkerEnvelope<InviteMemberInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        let project_id = project_request(
            &request,
            PROJECT_COLLABORATION_INVITE_MEMBER_OPERATION,
            request.payload.project_id,
        )?;
        if request.payload.username_or_email.trim().is_empty() {
            return Err(WorkerFailure::invalid_request("username_or_email 不能为空"));
        }
        let user_id = user_id(&request)?;
        a4n_db::collaboration::invite_project_member(
            self.database.connection(),
            user_id,
            project_id,
            &request.payload.username_or_email,
            request.payload.role,
        )
        .await
        .map_err(collaboration_error)?;
        Ok(ProjectCollaborationAck {})
    }

    pub(super) async fn accept_invite(
        &self,
        request: WorkerEnvelope<InviteInput>,
    ) -> Result<ProjectCollaborationAck, WorkerFailure> {
        unscoped_request(&request, PROJECT_COLLABORATION_ACCEPT_INVITE_OPERATION)?;
        WorkerFailure::require_positive_id(request.payload.invite_id, "invite_id")?;
        a4n_db::collaboration::accept_project_invite(
            self.database.connection(),
            user_id(&request)?,
            request.payload.invite_id,
        )
        .await
        .map_err(collaboration_error)?;
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
            user_id(&request)?,
            request.payload.invite_id,
        )
        .await
        .map_err(collaboration_error)?;
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
            user_id(&request)?,
            project_id,
            request.payload.invite_id,
        )
        .await
        .map_err(collaboration_error)?;
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
            user_id(&request)?,
            project_id,
            request.payload.member_user_id,
            request.payload.role,
        )
        .await
        .map_err(collaboration_error)?;
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
            user_id(&request)?,
            project_id,
            request.payload.member_user_id,
        )
        .await
        .map_err(collaboration_error)?;
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
            user_id(&request)?,
            project_id,
        )
        .await
        .map_err(collaboration_error)?;
        Ok(ProjectCollaborationAck {})
    }
}
