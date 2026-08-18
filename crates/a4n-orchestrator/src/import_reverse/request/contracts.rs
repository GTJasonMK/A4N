//! 导入反推请求输入合同。

use std::collections::HashMap;

use serde_json::Value;

use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use a4n_prompt::rendering::PromptRenderer;

pub struct ReverseEngineerRequestInput<'a> {
    pub project_id: i64,
    pub target: ReverseEngineeringTarget,
    pub content_sample: &'a str,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
}
