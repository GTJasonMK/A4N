use std::collections::HashMap;

use a4n_core::models::chapter::part_outline;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

pub(super) struct GeneratedPartOutline {
    pub(super) data: Value,
    pub(super) request: ChatRequest,
    pub(super) response_content: String,
    pub(super) finish_reason: Option<String>,
}

pub(super) struct PartOutlineJsonInput<'a> {
    pub(super) project_id: i64,
    pub(super) part_num: i32,
    pub(super) start_ch: i32,
    pub(super) end_ch: i32,
    pub(super) chapters_per_part: i32,
    pub(super) requested_part_count: i32,
    pub(super) blueprint_context: &'a Value,
    pub(super) previous_parts: &'a [part_outline::Model],
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) variables: &'a HashMap<String, Value>,
    pub(super) workflow_run_id: Option<i64>,
}

pub(super) struct PartOutlineRequestInput<'a> {
    pub(super) project_id: i64,
    pub(super) part_num: i32,
    pub(super) start_ch: i32,
    pub(super) end_ch: i32,
    pub(super) chapters_per_part: i32,
    pub(super) requested_part_count: i32,
    pub(super) blueprint_context: &'a Value,
    pub(super) previous_parts: &'a [part_outline::Model],
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) variables: &'a HashMap<String, Value>,
}
