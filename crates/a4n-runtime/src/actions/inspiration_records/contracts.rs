use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) const INSPIRATION_VARIABLES_ERROR: &str = "variables 必须是 JSON 对象";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummary {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Value>,
    pub prompt_tokens: Option<i32>,
    pub completion_tokens: Option<i32>,
    pub chapter_number: Option<i32>,
    pub is_important: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConverseInput {
    pub project_id: i64,
    pub user_input: String,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewInspirationRequestInput {
    pub project_id: i64,
    pub user_input: String,
    pub variables: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConverseStreamInput {
    pub project_id: i64,
    pub message: String,
    pub task_id: Option<String>,
    pub variables: Option<Value>,
}
