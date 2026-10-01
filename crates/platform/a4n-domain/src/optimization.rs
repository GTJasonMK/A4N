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
