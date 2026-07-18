pub use a4n_orchestrator::admin::{
    AdminOverviewSummary, AdminUserSummary, DailyUsagePoint, InviteCodeSummary, ProjectSummary,
    TaskQueueSummary, ThemeInfo, UsageLogSummary, UsageTrendsInput,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateInviteCodeInput {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserRoleInput {
    pub role: String,
}
