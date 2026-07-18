use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeEventSummary {
    pub id: i64,
    pub profile_id: i64,
    pub chapter_number: i32,
    pub timestamp: Option<String>,
    pub category: String,
    pub key: String,
    pub operation: String,
    pub change_description: String,
    pub old_value: Value,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AddProfileAttributeInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModifyProfileAttributeInput {
    pub profile_id: i64,
    pub chapter_number: i32,
    pub category: String,
    pub key: String,
    pub new_value: Value,
    pub cause: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttributeHistoryInput {
    pub profile_id: i64,
    pub chapter_from: Option<i32>,
    pub chapter_to: Option<i32>,
    pub category: Option<String>,
    pub key: Option<String>,
    pub operation: Option<String>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeMutationResultSummary {
    pub event: AttributeEventSummary,
}
