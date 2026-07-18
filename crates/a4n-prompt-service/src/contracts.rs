use a4n_prompt::engine::{DependencyGraph, PromptEntry, PromptVersion};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptRenderPayload {
    pub(crate) template_name: String,
    #[serde(default)]
    pub(crate) variables: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub(crate) project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptRenderOutput {
    pub(crate) rendered: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptNamePayload {
    pub(crate) name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptUpdatePayload {
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) author: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptUpdateManyPayload {
    #[serde(default)]
    pub(crate) updates: Vec<PromptBatchUpdate>,
    pub(crate) author: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptBatchUpdate {
    pub(crate) name: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptValidatePayload {
    pub(crate) template_name: String,
    #[serde(default)]
    pub(crate) variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptValidateOutput {
    pub(crate) missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptRenderAbOutput {
    pub(crate) version_a: String,
    pub(crate) version_b: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptProjectOverridePayload {
    pub(crate) project_id: i64,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptHydrateOverridesPayload {
    #[serde(default)]
    pub(crate) overrides: Vec<PromptHydrateOverride>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptHydrateOverride {
    pub(crate) project_id: i64,
    pub(crate) name: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CountOutput {
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptListOutput {
    pub(crate) prompts: Vec<PromptEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptVersionHistoryOutput {
    pub(crate) versions: Vec<PromptVersion>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptVersionPayload {
    pub(crate) name: String,
    pub(crate) version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PromptVersionDiffPayload {
    pub(crate) name: String,
    pub(crate) version_a: String,
    pub(crate) version_b: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptVersionDiffOutput {
    pub(crate) diff: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PromptDependencyGraphOutput {
    pub(crate) graph: DependencyGraph,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct CategoryInfo {
    pub(crate) key: String,
    pub(crate) label: Option<String>,
    pub(crate) order: i32,
    pub(crate) description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_payload_rejects_unknown_fields() {
        let error = serde_json::from_value::<PromptNamePayload>(
            serde_json::json!({"name": "chapter", "legacy_name": "old"}),
        )
        .expect_err("unknown prompt payload fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_name`"));
    }
}
