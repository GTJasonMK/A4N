use serde::{Deserialize, Serialize};
use serde_json::Value;

mod transport;

pub(super) use transport::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptEntrySummary {
    pub name: String,
    pub title: String,
    pub content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub dependencies: Vec<String>,
    pub is_modified: bool,
    pub version: String,
    pub modified_at: String,
    #[serde(default)]
    pub used_by: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionSummary {
    pub version: String,
    pub timestamp: String,
    pub author: String,
    pub message: String,
    pub content_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyGraphOutput {
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbRenderResult {
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderInput {
    pub template_name: String,
    pub variables: Option<Value>,
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePromptInput {
    pub name: String,
    pub content: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetOverrideInput {
    pub project_id: i64,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveOverrideInput {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionDiffInput {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateVarsInput {
    pub template_name: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbRenderInput {
    pub template_name: String,
    pub variables: Option<Value>,
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryInfo {
    pub key: String,
    pub label: Option<String>,
    pub order: i32,
    pub description: Option<String>,
}
