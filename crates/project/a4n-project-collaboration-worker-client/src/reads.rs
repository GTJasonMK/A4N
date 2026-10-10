use super::*;

pub(super) async fn list_project_members(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectMemberSummary>, AppError> {
    let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
    client
        .command(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION,
            ProjectInput { project_id },
        )
        .await
}

pub(super) async fn list_project_invites(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectInviteSummary>, AppError> {
    let project_id = a4n_worker_client::positive_id("project_id", project_id)?;
    client
        .command(
            user_id,
            Some(project_id),
            PROJECT_COLLABORATION_LIST_INVITES_OPERATION,
            ProjectInput { project_id },
        )
        .await
}

pub(super) async fn list_my_project_invites(
    client: &ProjectCollaborationWorkerClient,
    user_id: i64,
) -> Result<Vec<ProjectInviteSummary>, AppError> {
    client
        .command(
            user_id,
            None,
            PROJECT_COLLABORATION_LIST_MY_INVITES_OPERATION,
            EmptyInput {},
        )
        .await
}
