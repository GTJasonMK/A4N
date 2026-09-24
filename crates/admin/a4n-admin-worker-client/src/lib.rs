//! Typed Host adapter for the administrative worker.

use a4n_admin_worker_contract::{
    ADMIN_ACTIVE_OPERATION, ADMIN_CREATE_INVITE_OPERATION, ADMIN_CREATE_USER_OPERATION,
    ADMIN_DATABASE_HEALTH_OPERATION, ADMIN_GENERATION_ANALYTICS_OPERATION, ADMIN_INVITES_OPERATION,
    ADMIN_OVERVIEW_OPERATION, ADMIN_PENDING_TASKS_OPERATION, ADMIN_PERMISSION_OPERATION,
    ADMIN_PROJECTS_OPERATION, ADMIN_TOKEN_SUMMARY_OPERATION, ADMIN_UPDATE_ROLE_OPERATION,
    ADMIN_USAGE_TRENDS_OPERATION, ADMIN_USER_USAGE_OPERATION, ADMIN_USERS_OPERATION,
    ADMIN_WORKER_ID, AdminCallerInput, AdminCreateInviteInput, AdminCreateUserInput,
    AdminGenerationAnalyticsInput, AdminPermissionInput, AdminUpdateRoleInput,
    AdminUsageTrendsInput, AdminUserInput, AdminUserUsageInput,
};
use a4n_api_contract::admin::{
    AdminMutationApplicationPort, AdminOverviewSummary, AdminPermission, AdminProjectSummary,
    AdminReadApplicationPort, CreateInviteCodeInput, CreateUserInput, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, InviteCodeSummary, TaskQueueSummary,
    UpdateUserRoleInput, UsageLogSummary, UsageTrendsInput,
};
use a4n_api_contract::auth::UserSummary;
use a4n_error::AppError;
use a4n_worker_client::{HttpWorkerClient, WorkerClientError};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerLease,
    WorkerOperationStatus, WorkerPrincipal, WorkerPrincipalKind, WorkerResponse,
};

const DEFAULT_WORKER_URL: &str = "http://127.0.0.1:47679";
const CONTROL_LEASE_FENCE: &str = "control-request";
const CONTROL_LEASE_EXPIRY: &str = "9999-12-31T23:59:59Z";

#[derive(Clone)]
pub struct AdminWorkerClient {
    transport: HttpWorkerClient,
}

impl std::fmt::Debug for AdminWorkerClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdminWorkerClient")
            .field("transport", &self.transport)
            .finish()
    }
}

impl AdminWorkerClient {
    pub fn new(base_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let transport = HttpWorkerClient::new(base_url, ADMIN_WORKER_ID);
        let transport = match auth_token {
            Some(token) => transport.with_auth_token(token),
            None => transport,
        };
        Self { transport }
    }

    pub fn from_env() -> Self {
        let url = std::env::var("A4N_ADMIN_WORKER_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_WORKER_URL.to_string());
        let token = std::env::var("A4N_ADMIN_WORKER_TOKEN")
            .ok()
            .filter(|value| !value.trim().is_empty());
        Self::new(url, token)
    }

    async fn command<I, O>(
        &self,
        caller_user_id: i64,
        operation: &str,
        payload: I,
    ) -> Result<O, AppError>
    where
        I: serde::Serialize,
        O: serde::de::DeserializeOwned,
    {
        let request_id = uuid::Uuid::new_v4().to_string();
        let request = WorkerEnvelope {
            schema_version: WORKER_SCHEMA_VERSION,
            request_id: request_id.clone(),
            trace_id: request_id,
            principal: WorkerPrincipal {
                kind: WorkerPrincipalKind::User,
                subject: caller_user_id.to_string(),
            },
            operation_id: operation.into(),
            idempotency_key: uuid::Uuid::new_v4().to_string(),
            project_id: None,
            root_project_id: None,
            target_revision: None,
            runtime_config_snapshot_ref: None,
            lease: WorkerLease {
                worker_id: ADMIN_WORKER_ID.into(),
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
                AppError::ConfigError(format!("admin worker 完成响应缺少 payload: {operation}"))
            }),
            WorkerOperationStatus::Failed => Err(worker_response_error(
                response.error.as_ref().expect("validated failed response"),
                operation,
            )),
            status => Err(AppError::ConfigError(format!(
                "admin worker 返回未完成状态: operation={operation}, status={status:?}"
            ))),
        }
    }
}

#[async_trait::async_trait]
impl AdminReadApplicationPort for AdminWorkerClient {
    async fn require_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), AppError> {
        self.command::<_, a4n_admin_worker_contract::OperationAck>(
            caller_user_id,
            ADMIN_PERMISSION_OPERATION,
            AdminPermissionInput {
                caller_user_id,
                permission,
            },
        )
        .await
        .map(|_| ())
    }

    async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, AppError> {
        self.command(
            user_id,
            ADMIN_ACTIVE_OPERATION,
            AdminUserInput {
                caller_user_id: user_id,
                user_id,
            },
        )
        .await
    }

    async fn user_usage(
        &self,
        caller_user_id: i64,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_USER_USAGE_OPERATION,
            AdminUserUsageInput {
                caller_user_id,
                user_id,
                limit,
            },
        )
        .await
    }

    async fn token_summary(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<serde_json::Value, AppError> {
        self.command(
            caller_user_id,
            ADMIN_TOKEN_SUMMARY_OPERATION,
            AdminUserInput {
                caller_user_id,
                user_id,
            },
        )
        .await
    }

    async fn usage_trends(
        &self,
        caller_user_id: i64,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_USAGE_TRENDS_OPERATION,
            AdminUsageTrendsInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn overview(&self, caller_user_id: i64) -> Result<AdminOverviewSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_OVERVIEW_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn all_project_summaries(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<AdminProjectSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_PROJECTS_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn pending_tasks(&self, caller_user_id: i64) -> Result<Vec<TaskQueueSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_PENDING_TASKS_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn generation_call_analytics(
        &self,
        caller_user_id: i64,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, AppError> {
        self.command(
            caller_user_id,
            ADMIN_GENERATION_ANALYTICS_OPERATION,
            AdminGenerationAnalyticsInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn check_database_health(&self, caller_user_id: i64) -> Result<(), AppError> {
        self.command::<_, a4n_admin_worker_contract::OperationAck>(
            caller_user_id,
            ADMIN_DATABASE_HEALTH_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
        .map(|_| ())
    }

    async fn list_users(&self, caller_user_id: i64) -> Result<Vec<UserSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_USERS_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }

    async fn list_invite_codes(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<InviteCodeSummary>, AppError> {
        self.command(
            caller_user_id,
            ADMIN_INVITES_OPERATION,
            AdminCallerInput { caller_user_id },
        )
        .await
    }
}

#[async_trait::async_trait]
impl AdminMutationApplicationPort for AdminWorkerClient {
    async fn create_user(
        &self,
        caller_user_id: i64,
        input: CreateUserInput,
    ) -> Result<UserSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_CREATE_USER_OPERATION,
            AdminCreateUserInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn create_invite_code(
        &self,
        caller_user_id: i64,
        input: CreateInviteCodeInput,
    ) -> Result<InviteCodeSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_CREATE_INVITE_OPERATION,
            AdminCreateInviteInput {
                caller_user_id,
                input,
            },
        )
        .await
    }

    async fn update_user_role(
        &self,
        caller_user_id: i64,
        user_id: i64,
        input: UpdateUserRoleInput,
    ) -> Result<UserSummary, AppError> {
        self.command(
            caller_user_id,
            ADMIN_UPDATE_ROLE_OPERATION,
            AdminUpdateRoleInput {
                caller_user_id,
                user_id,
                input,
            },
        )
        .await
    }
}

fn worker_client_error(error: WorkerClientError) -> AppError {
    AppError::ConfigError(format!("admin worker 请求失败: {error}"))
}

fn worker_response_error(error: &WorkerError, operation: &str) -> AppError {
    match error.code {
        WorkerErrorCode::InvalidRequest => AppError::InvalidArgument(error.message.clone()),
        WorkerErrorCode::Unauthorized => AppError::Forbidden(error.message.clone()),
        WorkerErrorCode::NotFound => AppError::NotFound(error.message.clone()),
        WorkerErrorCode::Conflict => AppError::InvalidState {
            current: error.message.clone(),
            operation: operation.to_string(),
        },
        _ => AppError::ConfigError(format!("admin worker {operation} 失败: {}", error.message)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_debug_redacts_worker_address_and_token() {
        let client = AdminWorkerClient::new(
            "https://admin-worker.example.test",
            Some("admin-token".into()),
        );
        let debug = format!("{client:?}");
        assert!(!debug.contains("admin-worker.example.test"));
        assert!(!debug.contains("admin-token"));
    }
}
