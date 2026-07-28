use super::generation_tasks::GenerationStartRequestMeta;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Start a structured blueprint generation through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartBlueprintGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 1024))]
    pub title: String,
    #[schemars(length(min = 1, max = 256))]
    pub genre: String,
    #[schemars(range(min = 1))]
    pub target_length: i32,
    pub variables: Option<BTreeMap<String, Value>>,
}

/// Start a structured blueprint refinement through the canonical generation task path.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartBlueprintRefinementInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(max = 32768))]
    pub focus: Option<String>,
    pub variables: Option<BTreeMap<String, Value>>,
}
