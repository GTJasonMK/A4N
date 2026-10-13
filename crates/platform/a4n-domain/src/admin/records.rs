use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    ThemeMode,
    project::{ProjectKind, ProjectStatus},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageTrendsInput {
    #[schemars(range(min = 1))]
    pub days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DailyUsagePoint {
    pub date: String,
    pub total_calls: usize,
    pub total_tokens: i64,
    pub success_count: usize,
    pub fail_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ThemeInfo {
    pub id: i64,
    pub name: String,
    pub mode: ThemeMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AdminOverviewSummary {
    pub total_users: u64,
    pub total_projects: u64,
    pub total_chapters: u64,
    pub task_queue_count: u64,
    pub pending_tasks: u64,
    pub running_tasks: u64,
    pub total_llm_calls: u64,
    pub llm_config_count: u64,
    pub embedding_config_count: u64,
    pub image_gen_config_count: u64,
    pub active_theme: Option<ThemeInfo>,
    pub daily_active_users: u64,
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
pub struct PendingTaskIndexSummary {
    pub id: String,
    pub task_type: String,
    pub priority: String,
    pub status: String,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub dag_id: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AdminProjectSummary {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub project_type: String,
    pub project_kind: ProjectKind,
    pub status: ProjectStatus,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
}
