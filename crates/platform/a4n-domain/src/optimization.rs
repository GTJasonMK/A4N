use chrono::NaiveDateTime;
use serde_json::Value;

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
