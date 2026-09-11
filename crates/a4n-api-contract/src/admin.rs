use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Permissions used by host-facing administrative commands.
///
/// The enum lives with the host contract so Runtime and other Hosts do not
/// import the orchestrator's authorization implementation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AdminPermission {
    ManageUsers,
    ManageSettings,
}

pub use a4n_domain::admin::{
    AdminOverviewSummary, AdminProjectSummary, CallKindOverview, DailyCallMetric, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, InviteCodeSummary, LlmWorkflowMetric,
    MediaWorkflowMetric, RecentLlmCall, RecentMediaCall, TaskQueueSummary, ThemeInfo,
    UsageLogSummary, UsageTrendsInput,
};
pub use a4n_domain::auth::UserSummary;

/// Host-facing administrative application boundary.
///
/// Database queries and user-management mutations stay in the administrative
/// owner. Runtime keeps only the command authorization wrapper and lifecycle
/// operations that belong to the queue or service manager.
#[async_trait::async_trait]
pub trait AdminApplicationPort: Send + Sync {
    async fn require_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), a4n_error::AppError>;

    async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, a4n_error::AppError>;

    async fn user_usage(
        &self,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, a4n_error::AppError>;

    async fn token_summary(&self, user_id: i64) -> Result<serde_json::Value, a4n_error::AppError>;

    async fn usage_trends(
        &self,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, a4n_error::AppError>;

    async fn overview(&self) -> Result<AdminOverviewSummary, a4n_error::AppError>;

    async fn all_project_summaries(&self) -> Result<Vec<AdminProjectSummary>, a4n_error::AppError>;

    async fn pending_tasks(&self) -> Result<Vec<TaskQueueSummary>, a4n_error::AppError>;

    async fn generation_call_analytics(
        &self,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, a4n_error::AppError>;

    async fn check_database_health(&self) -> Result<(), a4n_error::AppError>;

    async fn list_users(&self) -> Result<Vec<UserSummary>, a4n_error::AppError>;

    async fn create_user(&self, input: CreateUserInput)
    -> Result<UserSummary, a4n_error::AppError>;

    async fn list_invite_codes(&self) -> Result<Vec<InviteCodeSummary>, a4n_error::AppError>;

    async fn create_invite_code(
        &self,
        caller_user_id: i64,
        input: CreateInviteCodeInput,
    ) -> Result<InviteCodeSummary, a4n_error::AppError>;

    async fn update_user_role(
        &self,
        user_id: i64,
        input: UpdateUserRoleInput,
    ) -> Result<UserSummary, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateInviteCodeInput {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserRoleInput {
    pub role: String,
}
