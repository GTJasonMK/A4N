use std::collections::HashMap;

use serde::{Deserialize, Serialize};
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
    pub current: String,
    pub original: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptSetProjectOverridePayload {
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptHydrateOverridesPayload {
    pub overrides: Vec<PromptHydrateOverride>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptHydrateOverride {
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
    pub versions: Vec<PromptVersionSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionSummary {
    pub version: String,
    pub timestamp: chrono::NaiveDateTime,
    pub author: String,
    pub message: String,
}

impl From<PromptVersion> for PromptVersionSummary {
    fn from(version: PromptVersion) -> Self {
        Self {
            version: version.version,
            timestamp: version.timestamp,
            author: version.author,
            message: version.message,
        }
    }
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

        serde_json::from_value::<PromptSetProjectOverridePayload>(serde_json::json!({
            "name": "writing"
        }))
        .expect_err("set payload must require content");
        serde_json::from_value::<PromptNamePayload>(serde_json::json!({
            "name": "writing",
            "content": "legacy"
        }))
        .expect_err("remove payload must reject content");
    }

    #[test]
    fn current_producers_must_send_complete_collections() {
        serde_json::from_value::<PromptValidatePayload>(serde_json::json!({
            "template_name": "writing"
        }))
        .expect_err("validate payload must provide variables");
        serde_json::from_value::<PromptHydrateOverridesPayload>(serde_json::json!({}))
            .expect_err("hydrate payload must provide overrides");
    }
}
