use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::engine::{PromptEntry, PromptVersion};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptEmptyPayload {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptRenderPayload {
    pub template_name: String,
    pub variables: HashMap<String, Value>,
    #[serde(deserialize_with = "deserialize_required_project_id")]
    pub project_id: Option<i64>,
}

fn deserialize_required_project_id<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<i64>::deserialize(deserializer)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptRenderOutput {
    pub rendered: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptNamePayload {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptUpdatePayload {
    pub name: String,
    pub content: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptUpdateManyPayload {
    pub updates: Vec<PromptBatchUpdate>,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptBatchUpdate {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptValidatePayload {
    pub template_name: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptValidateOutput {
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptRenderAbOutput {
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptSetProjectOverridePayload {
    pub project_id: i64,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptRemoveProjectOverridePayload {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptHydrateOverridesPayload {
    pub overrides: Vec<PromptHydrateOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptHydrateOverride {
    pub project_id: i64,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountOutput {
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptListOutput {
    pub prompts: Vec<PromptEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionHistoryOutput {
    pub versions: Vec<PromptVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionDiffPayload {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionDiffOutput {
    pub diff: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_payloads_are_strict_and_route_specific() {
        let unknown = serde_json::from_value::<PromptNamePayload>(
            serde_json::json!({"name": "chapter", "legacy_name": "old"}),
        )
        .expect_err("unknown prompt payload fields must fail");
        assert!(unknown.to_string().contains("unknown field `legacy_name`"));

        serde_json::from_value::<PromptSetProjectOverridePayload>(
            serde_json::json!({"project_id": 42, "name": "writing"}),
        )
        .expect_err("set payload must require content");
        serde_json::from_value::<PromptRemoveProjectOverridePayload>(serde_json::json!({
            "project_id": 42,
            "name": "writing",
            "content": "legacy"
        }))
        .expect_err("remove payload must reject content");
    }

    #[test]
    fn current_producers_must_send_complete_collections_and_render_scope() {
        serde_json::from_value::<PromptRenderPayload>(serde_json::json!({
            "template_name": "writing",
            "variables": {}
        }))
        .expect_err("render payload must state its project scope");
        serde_json::from_value::<PromptValidatePayload>(serde_json::json!({
            "template_name": "writing"
        }))
        .expect_err("validate payload must provide variables");
        serde_json::from_value::<PromptHydrateOverridesPayload>(serde_json::json!({}))
            .expect_err("hydrate payload must provide overrides");
    }
}
