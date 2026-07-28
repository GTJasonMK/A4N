use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReverseEngineerInput {
    pub project_id: i64,
    #[serde(default)]
    pub content_sample: String,
    #[serde(default)]
    pub auto_context: bool,
    #[serde(default)]
    pub context_limit: Option<usize>,
    pub variables: Option<Value>,
}
