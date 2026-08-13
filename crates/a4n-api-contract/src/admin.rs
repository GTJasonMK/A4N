use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::admin::{
    AdminOverviewSummary, AdminProjectSummary, CallKindOverview, DailyCallMetric, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, InviteCodeSummary, LlmWorkflowMetric,
    MediaWorkflowMetric, RecentLlmCall, RecentMediaCall, TaskQueueSummary, ThemeInfo,
    UsageLogSummary, UsageTrendsInput,
};

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
