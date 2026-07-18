use a4n_core::error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::json;

pub(in crate::import_reverse::request) use self::source_artifact::reverse_engineer_source_artifact;
use super::output_contract::reverse_output_contract;

mod source_artifact;
mod validation;

use validation::required_reverse_text;

pub fn build_reverse_engineer_user_message(
    prompt_name: &str,
    instruction: &str,
) -> Result<String, AppError> {
    let instruction = required_reverse_text(instruction, "instruction")?;

    let payload = json!({
        "context_refs": {
            "source": "稳定上下文包 source_artifact",
        },
        "task": {
            "prompt_name": prompt_name,
            "instruction": instruction,
        },
        "output_contract": reverse_output_contract(prompt_name),
    });
    build_json_task_user_message(
        "执行导入反推",
        "只返回系统提示词要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}
