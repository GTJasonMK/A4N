//! Typed Host adapter for the project-collaboration worker.

use a4n_api_contract::collaboration::{
    InviteProjectMemberInput, ProjectCollaborationApplicationPort, ProjectInviteSummary,
    ProjectMemberSummary, UpdateProjectMemberRoleInput,
};
use a4n_error::AppError;
use a4n_project_collaboration_worker_contract::{
    EmptyInput, InviteInput, InviteMemberInput, MemberInput,
    PROJECT_COLLABORATION_ACCEPT_INVITE_OPERATION, PROJECT_COLLABORATION_DECLINE_INVITE_OPERATION,
    PROJECT_COLLABORATION_INVITE_MEMBER_OPERATION, PROJECT_COLLABORATION_LEAVE_OPERATION,
    PROJECT_COLLABORATION_LIST_INVITES_OPERATION, PROJECT_COLLABORATION_LIST_MEMBERS_OPERATION,
    PROJECT_COLLABORATION_LIST_MY_INVITES_OPERATION, PROJECT_COLLABORATION_REMOVE_MEMBER_OPERATION,
    PROJECT_COLLABORATION_REVOKE_INVITE_OPERATION, PROJECT_COLLABORATION_UPDATE_ROLE_OPERATION,
    PROJECT_COLLABORATION_WORKER_ID, ProjectCollaborationAck, ProjectInput, ProjectInviteInput,
};
use a4n_worker_client::{
    HttpWorkerClient, WorkerClientErrorPolicy, map_worker_client_error, map_worker_response_error,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerLease, WorkerOperationStatus,
    WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47668";

mod mutations;
mod reads;

#[derive(Clone, Debug)]
pub struct ProjectCollaborationWorkerClient {
    transport: HttpWorkerClient,
}

impl ProjectCollaborationWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, PROJECT_COLLABORATION_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let (url, token) = a4n_worker_client::worker_client_connection_from_env(
            "A4N_PROJECT_COLLABORATION_WORKER_URL",
            "A4N_PROJECT_COLLABORATION_WORKER_TOKEN",
            DEFAULT_WORKER_URL,
        );
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let principal = a4n_worker_client::user_principal(user_id)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal,
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease::control_request(PROJECT_COLLABORATION_WORKER_ID),
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(|error| {
                map_worker_client_error(
                    error,
                    "project collaboration worker",
                    WorkerClientErrorPolicy::Configuration,
                )
            })?;
        match response.status {
            WorkerOperationStatus::Completed => response.payload.ok_or_else(|| {
                AppError::ConfigError(format!(
                    "project collaboration worker 完成响应缺少 payload: {operation}"
                ))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "project collaboration worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }

    fn project_id(project_id: i64) -> Result<i64, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("project_id 必须大于 0".into()));
        }
        Ok(project_id)
    }

    fn invite_id(invite_id: i64) -> Result<i64, AppError> {
        if invite_id <= 0 {
            return Err(AppError::InvalidArgument("invite_id 必须大于 0".into()));
        }
        Ok(invite_id)
    }

    fn member_id(member_user_id: i64) -> Result<i64, AppError> {
        if member_user_id <= 0 {
            return Err(AppError::InvalidArgument(
                "member_user_id 必须大于 0".into(),
            ));
        }
        Ok(member_user_id)
    }

    async fn unit<I>(
        &self,
        user_id: i64,
        project_id: Option<i64>,
        operation: &str,
        payload: I,
    ) -> Result<(), AppError>
    where
        I: serde::Serialize,
    {
        let _: ProjectCollaborationAck = self
            .command(user_id, project_id, operation, payload)
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ProjectCollaborationApplicationPort for ProjectCollaborationWorkerClient {
    async fn list_project_members(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, AppError> {
        reads::list_project_members(self, user_id, project_id).await
    }

    async fn invite_project_member(
        &self,
        user_id: i64,
        project_id: i64,
        input: InviteProjectMemberInput,
    ) -> Result<(), AppError> {
        mutations::invite_project_member(self, user_id, project_id, input).await
    }

    async fn list_project_invites(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        reads::list_project_invites(self, user_id, project_id).await
    }

    async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        reads::list_my_project_invites(self, user_id).await
    }

    async fn accept_project_invite(&self, user_id: i64, invite_id: i64) -> Result<(), AppError> {
        mutations::accept_project_invite(self, user_id, invite_id).await
    }

    async fn decline_project_invite(&self, user_id: i64, invite_id: i64) -> Result<(), AppError> {
        mutations::decline_project_invite(self, user_id, invite_id).await
    }

    async fn revoke_project_invite(
        &self,
        user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<(), AppError> {
        mutations::revoke_project_invite(self, user_id, project_id, invite_id).await
    }

    async fn update_project_member_role(
        &self,
        user_id: i64,
        project_id: i64,
        member_user_id: i64,
        input: UpdateProjectMemberRoleInput,
    ) -> Result<(), AppError> {
        mutations::update_project_member_role(self, user_id, project_id, member_user_id, input)
            .await
    }

    async fn remove_project_member(
        &self,
        user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), AppError> {
        mutations::remove_project_member(self, user_id, project_id, member_user_id).await
    }

    async fn leave_project(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        mutations::leave_project(self, user_id, project_id).await
    }
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    map_worker_response_error(error, "project collaboration worker", operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = ProjectCollaborationWorkerClient::new(
            "https://project-collaboration-worker.example.test",
            Some("project-collaboration-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("project-collaboration-worker.example.test"));
        assert!(!debug.contains("project-collaboration-token"));
    }
}
