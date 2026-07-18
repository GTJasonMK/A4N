use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DefaultLlmOutputCapRow {
    pub id: i64,
    pub name: String,
    pub max_output_tokens: i32,
}

#[derive(Clone, Debug)]
pub struct LlmConfigCreateRecordInput {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub request_options_json: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct LlmConfigUpdateRecordInput {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    pub request_options_json: Option<Value>,
}
