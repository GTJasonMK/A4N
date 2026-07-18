//! 导入反推请求输入合同。

use std::collections::HashMap;

use serde_json::Value;

use a4n_prompt::rendering::PromptRenderer;

pub struct ReverseEngineerRequestInput<'a> {
    pub project_id: i64,
    pub prompt_name: &'a str,
    pub user_prefix: &'a str,
    pub content_sample: &'a str,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub max_tokens_profile: &'a str,
}
