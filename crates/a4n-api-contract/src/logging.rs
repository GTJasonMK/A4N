use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SetLogLevelInput {
    pub filter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReadLogsInput {
    pub lines: Option<usize>,
    pub days_ago: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct LogReadResult {
    pub file_path: String,
    pub total_lines: usize,
    pub returned_lines: usize,
    pub lines: Vec<String>,
}
