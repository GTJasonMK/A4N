use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OptimizationPatch {
    pub patch_id: String,
    pub issue_id: Option<String>,
    pub operation: String,
    pub old_text: String,
    pub new_text: String,
    pub reason: String,
    pub risk: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizationPatchProposal {
    pub session_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub base_content_hash: String,
    pub summary: String,
    pub patches: Vec<OptimizationPatch>,
}
