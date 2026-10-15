use super::*;

pub(super) async fn invite_project_member(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
    input: InviteProjectMemberInput,
) -> Result<ProjectInviteSummary, AppError> {
    let project_id = ProjectCollaborationWorkerClient::project_id(project_id)?;
    if input.username_or_email.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "username_or_email 不能为空".into(),
        ));
    }
    client
        .command(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_INVITE_MEMBER_OPERATION,
            InviteMemberInput {
                project_id,
                username_or_email: input.username_or_email,
                role: input.role,
            },
        )
        .await
}

pub(super) async fn accept_project_invite(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    invite_id: i64,
) -> Result<(), AppError> {
    client
        .unit(
            user_id,
            None,
            PROJECT_COLLABORATION_ACCEPT_INVITE_OPERATION,
            InviteInput {
                invite_id: a4n_worker_client::positive_id("invite_id", invite_id)?,
            },
        )
        .await
}

pub(super) async fn decline_project_invite(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    invite_id: i64,
) -> Result<(), AppError> {
    client
        .unit(
            user_id,
            None,
            PROJECT_COLLABORATION_DECLINE_INVITE_OPERATION,
            InviteInput {
                invite_id: a4n_worker_client::positive_id("invite_id", invite_id)?,
            },
        )
        .await
}

pub(super) async fn revoke_project_invite(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
    invite_id: i64,
) -> Result<(), AppError> {
    let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
    client
        .unit(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_REVOKE_INVITE_OPERATION,
            ProjectInviteInput {
                project_id,
                invite_id: a4n_worker_client::positive_id("invite_id", invite_id)?,
            },
        )
        .await
}

pub(super) async fn update_project_member_role(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
    member_user_id: i64,
    input: UpdateProjectMemberRoleInput,
) -> Result<(), AppError> {
    let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
    client
        .unit(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_UPDATE_ROLE_OPERATION,
            a4n_project_collaboration_worker_contract::UpdateMemberRoleInput {
                project_id,
                member_user_id: a4n_worker_client::positive_id("member_user_id", member_user_id)?,
                role: input.role,
            },
        )
        .await
}

pub(super) async fn remove_project_member(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
    member_user_id: i64,
) -> Result<(), AppError> {
    let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
    client
        .unit(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_REMOVE_MEMBER_OPERATION,
            MemberInput {
                project_id,
                member_user_id: a4n_worker_client::positive_id("member_user_id", member_user_id)?,
            },
        )
        .await
}

pub(super) async fn leave_project(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
    client
        .unit(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_LEAVE_OPERATION,
            ProjectInput { project_id },
        )
        .await
}
