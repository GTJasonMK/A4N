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
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47668";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

mod mutations;
mod reads;

#[derive(Clone)]
pub struct ProjectCollaborationWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for ProjectCollaborationWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProjectCollaborationWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
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
        let url = std::env::var("A4N_PROJECT_COLLABORATION_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_PROJECT_COLLABORATION_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
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
        let principal = if user_id > 0 {
            WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: user_id.to_string(),
            }
        } else {
            return Err(AppError::InvalidArgument("user_id 必须大于 0".into()));
        };
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
            lease: WorkerLease {
                worker_id: PROJECT_COLLABORATION_WORKER_ID.into(),
                epoch: 1,
                fence_token: CONTROL_LEASE_FENCE.into(),
                expires_at: CONTROL_LEASE_EXPIRY.into(),
            },
            payload,
        };
        let response: WorkerResponse<O> = self
            .transport
            .command(operation, request)
            .await
            .map_err(worker_client_error)?;
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

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("project collaboration worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.into(),
        },
        _ => AppError::ConfigError(format!(
            "project collaboration worker {operation} 失败: {}",
            error.message
        )),
    }
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
