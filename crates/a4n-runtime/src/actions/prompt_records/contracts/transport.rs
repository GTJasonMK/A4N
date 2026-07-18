use a4n_prompt::engine::{DependencyGraph, PromptEntry, PromptVersion};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptBatchUpdate {
    pub(in crate::actions::prompt_records) name: String,
    pub(in crate::actions::prompt_records) content: String,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptUpdateManyPayload {
    pub(in crate::actions::prompt_records) updates: Vec<PromptBatchUpdate>,
    pub(in crate::actions::prompt_records) author: String,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptRenderPayload {
    pub(in crate::actions::prompt_records) template_name: String,
    pub(in crate::actions::prompt_records) variables: HashMap<String, Value>,
    pub(in crate::actions::prompt_records) project_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptRenderOutput {
    pub(in crate::actions::prompt_records) rendered: String,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptNamePayload {
    pub(in crate::actions::prompt_records) name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptListOutput {
    pub(in crate::actions::prompt_records) prompts: Vec<PromptEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptVersionHistoryOutput {
    pub(in crate::actions::prompt_records) versions: Vec<PromptVersion>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptVersionDiffPayload {
    pub(in crate::actions::prompt_records) name: String,
    pub(in crate::actions::prompt_records) version_a: String,
    pub(in crate::actions::prompt_records) version_b: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptVersionDiffOutput {
    pub(in crate::actions::prompt_records) diff: String,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptValidatePayload {
    pub(in crate::actions::prompt_records) template_name: String,
    pub(in crate::actions::prompt_records) variables: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptValidateOutput {
    pub(in crate::actions::prompt_records) missing: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptRenderAbOutput {
    pub(in crate::actions::prompt_records) version_a: String,
    pub(in crate::actions::prompt_records) version_b: String,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptProjectOverridePayload {
    pub(in crate::actions::prompt_records) project_id: i64,
    pub(in crate::actions::prompt_records) name: String,
    pub(in crate::actions::prompt_records) content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptHydrateOverridesPayload {
    pub(in crate::actions::prompt_records) overrides: Vec<PromptHydrateOverride>,
}

#[derive(Debug, Clone, Serialize)]
pub(in crate::actions::prompt_records) struct PromptHydrateOverride {
    pub(in crate::actions::prompt_records) project_id: i64,
    pub(in crate::actions::prompt_records) name: String,
    pub(in crate::actions::prompt_records) content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct CountOutput {
    pub(in crate::actions::prompt_records) count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::actions::prompt_records) struct PromptDependencyGraphOutput {
    pub(in crate::actions::prompt_records) graph: DependencyGraph,
}

#[cfg(test)]
mod tests {
    use super::PromptRenderOutput;

    #[test]
    fn prompt_output_wrappers_reject_unknown_fields() {
        let error = serde_json::from_value::<PromptRenderOutput>(serde_json::json!({
            "rendered": "text",
            "legacy_template": "old"
        }))
        .expect_err("prompt output must reject unknown fields");
        assert!(error.to_string().contains("legacy_template"));
    }
}
