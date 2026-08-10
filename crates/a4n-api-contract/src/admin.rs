use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::project::ProjectStatus;

pub use a4n_domain::admin::{
    AdminOverviewSummary, CallKindOverview, DailyCallMetric, DailyUsagePoint,
    GenerationCallAnalytics, GenerationCallAnalyticsInput, LlmWorkflowMetric, MediaWorkflowMetric,
    RecentLlmCall, RecentMediaCall, ThemeInfo, UsageTrendsInput,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InviteCodeSummary {
    pub id: i64,
    pub code: String,
    pub created_by_user_id: Option<i64>,
    pub used_by_user_id: Option<i64>,
    pub created_at: Option<String>,
    pub used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct UsageLogSummary {
    pub id: i64,
    pub user_id: i64,
    pub project_id: Option<i64>,
    pub model: String,
    pub operation: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub latency_ms: i32,
    pub is_success: bool,
    pub error_message: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct TaskQueueSummary {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub payload: Option<Value>,
    pub progress: Option<Value>,
    pub error_message: Option<String>,
    pub dag_id: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AdminProjectSummary {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub status: ProjectStatus,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub avatar_animal: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}
