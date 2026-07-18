use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::optimize) struct OptimizeIssue {
    pub id: String,
    pub tool: String,
    pub arguments: Value,
    pub description: String,
    pub severity: String,
}
