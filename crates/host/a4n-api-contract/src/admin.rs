use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Host-facing administrative read and authorization boundary.
///
/// Database queries and user-management mutations stay in the administrative
/// owner.
#[async_trait::async_trait]
pub trait AdminReadApplicationPort: Send + Sync {
    async fn require_permission(
        &self,
        caller_user_id: i64,
        permission: AdminPermission,
    ) -> Result<(), a4n_error::AppError>;

    async fn is_active_admin_user(&self, user_id: i64) -> Result<bool, a4n_error::AppError>;

    async fn user_usage(
        &self,
        caller_user_id: i64,
        user_id: i64,
        limit: u64,
    ) -> Result<Vec<UsageLogSummary>, a4n_error::AppError>;

    async fn token_summary(
        &self,
        caller_user_id: i64,
        user_id: i64,
    ) -> Result<serde_json::Value, a4n_error::AppError>;

    async fn usage_trends(
        &self,
        caller_user_id: i64,
        input: UsageTrendsInput,
    ) -> Result<Vec<DailyUsagePoint>, a4n_error::AppError>;

    async fn overview(
        &self,
        caller_user_id: i64,
    ) -> Result<AdminOverviewSummary, a4n_error::AppError>;

    async fn all_project_summaries(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<AdminProjectSummary>, a4n_error::AppError>;

    async fn pending_tasks(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<TaskQueueSummary>, a4n_error::AppError>;

    async fn generation_call_analytics(
        &self,
        caller_user_id: i64,
        input: GenerationCallAnalyticsInput,
    ) -> Result<GenerationCallAnalytics, a4n_error::AppError>;

    async fn check_database_health(&self, caller_user_id: i64) -> Result<(), a4n_error::AppError>;

    async fn list_users(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<UserSummary>, a4n_error::AppError>;

    async fn list_invite_codes(
        &self,
        caller_user_id: i64,
    ) -> Result<Vec<InviteCodeSummary>, a4n_error::AppError>;
}

/// Host-facing administrative mutation boundary.
#[async_trait::async_trait]
pub trait AdminMutationApplicationPort: Send + Sync {
    async fn create_user(
        &self,
        caller_user_id: i64,
        input: CreateUserInput,
    ) -> Result<UserSummary, a4n_error::AppError>;

    async fn create_invite_code(
        &self,
        caller_user_id: i64,
        input: CreateInviteCodeInput,
    ) -> Result<InviteCodeSummary, a4n_error::AppError>;

    async fn update_user_role(
        &self,
        caller_user_id: i64,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestScenarioEffect {
    Read,
    Write,
    Destructive,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestTargetMode {
    TestProject,
    CurrentProject,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TestRunStatus {
    Queued,
    Running,
    Passed,
    Failed,
    Blocked,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum TestTarget {
    TestProject { source_project_id: Option<i64> },
    CurrentProject { project_id: i64 },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TestScenarioDescriptor {
    pub id: String,
    pub version: u32,
    pub category: String,
    pub title: String,
    pub description: String,
    pub effect: TestScenarioEffect,
    pub target_modes: Vec<TestTargetMode>,
    pub timeout_seconds: u64,
    pub parameter_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartTestRunInput {
    pub scenario_id: String,
    pub target: TestTarget,
    #[serde(default)]
    pub parameters: Value,
    #[serde(default)]
    pub confirm_impact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TestImpact {
    pub action: String,
    pub resource: String,
    pub project_id: Option<i64>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TestRunSummary {
    pub id: String,
    pub scenario_id: String,
    pub scenario_version: u32,
    pub status: TestRunStatus,
    pub target: TestTarget,
    pub target_project_id: Option<i64>,
    pub created_by: i64,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub message: Option<String>,
    pub impact: Vec<TestImpact>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TestStepSummary {
    pub index: u32,
    pub name: String,
    pub status: TestRunStatus,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TestRunDetail {
    pub summary: TestRunSummary,
    pub parameters: Value,
    pub steps: Vec<TestStepSummary>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListTestRunsInput {
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TestRunIdInput {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DeleteTestRunInput {
    pub run_id: String,
    #[serde(default)]
    pub delete_test_project: bool,
}
