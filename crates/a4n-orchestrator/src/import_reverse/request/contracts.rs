//! 导入反推请求输入合同。

use std::collections::HashMap;

use serde_json::Value;

use a4n_api_contract::import_reverse::ReverseEngineeringTarget;
use a4n_prompt::rendering::PromptRenderer;

pub(in crate::import_reverse) struct ReverseEngineerRequestInput<'a> {
    pub(in crate::import_reverse) project_id: i64,
    pub(in crate::import_reverse) target: ReverseEngineeringTarget,
    pub(in crate::import_reverse) content_sample: &'a str,
    pub(in crate::import_reverse) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::import_reverse) variables: &'a HashMap<String, Value>,
}
