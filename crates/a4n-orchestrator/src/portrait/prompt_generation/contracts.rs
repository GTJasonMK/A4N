use std::collections::HashMap;

use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

#[derive(Clone, Copy)]
pub struct PortraitPromptSpec<'a> {
    pub profile_name: &'a str,
    pub appearance: Option<&'a str>,
    pub gender: Option<&'a str>,
    pub age: Option<&'a str>,
    pub personality: Option<&'a str>,
    pub background: Option<&'a str>,
    pub profile_context: Option<&'a Value>,
    pub project_context: Option<&'a Value>,
    pub portrait_type: &'a str,
    pub art_style: Option<&'a str>,
    pub expression: Option<&'a str>,
    pub existing_portrait_count: Option<u64>,
}

pub struct PortraitPromptRequestInput<'a> {
    pub project_id: i64,
    pub spec: PortraitPromptSpec<'a>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
}

pub struct PortraitPromptGenerationInput<'a> {
    pub project_id: i64,
    pub spec: PortraitPromptSpec<'a>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
}
