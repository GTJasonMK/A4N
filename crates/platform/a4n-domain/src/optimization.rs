use chrono::NaiveDateTime;
use serde_json::Value;

pub const OPTIMIZATION_PATCH_SET_STATUS_PENDING: &str = "pending";
pub const OPTIMIZATION_PATCH_SET_STATUS_APPLIED: &str = "applied";
pub const OPTIMIZATION_PATCH_SET_STATUS_REJECTED: &str = "rejected";
pub const OPTIMIZATION_PATCH_DECISION_ACCEPTED: &str = "accepted";
pub const OPTIMIZATION_PATCH_DECISION_PENDING: &str = "pending";
pub const OPTIMIZATION_PATCH_DECISION_REJECTED: &str = "rejected";
pub const OPTIMIZATION_PATCH_APPLY_STATUS_PENDING: &str = "pending";
pub const OPTIMIZATION_PATCH_APPLY_STATUS_APPLIED: &str = "applied";

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationSessionRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub name: Option<String>,
    pub status: String,
    pub issues: Option<Value>,
    pub agent_permissions: Option<Value>,
    pub agent_instruction: Option<String>,
    pub fixes_applied: i32,
    pub summary: Option<String>,
    pub archived_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationSessionMessageRecord {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationPatchSetRecord {
    pub id: i64,
    pub turn_id: i64,
    pub session_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub status: String,
    pub summary: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationPatchRecord {
    pub id: i64,
    pub patch_set_id: i64,
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: String,
    pub old_text: String,
    pub new_text: String,
    pub reason: String,
    pub risk: Option<String>,
    pub decision: String,
    pub apply_status: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationTurnRecord {
    pub id: i64,
    pub session_id: i64,
    pub agent_turn_id: Option<i64>,
    pub user_text: String,
    pub status: String,
    pub permissions_snapshot: Option<Value>,
    pub final_reply: Option<String>,
    pub error: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationTurnEventRecord {
    pub id: i64,
    pub turn_id: i64,
    pub seq: i32,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload: Option<Value>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProjectOptimizationContextRecord {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub status: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub raw_blueprint: Option<String>,
    pub export_config: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizationCharacterProfileRecord {
    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub base_attributes: Value,
    pub current_state: Value,
    pub character_arc: Option<Value>,
    pub voice_fingerprint: Option<Value>,
    pub appearance: Option<String>,
    pub background: Option<String>,
    pub personality: Option<String>,
    pub last_synced_chapter: i32,
}
