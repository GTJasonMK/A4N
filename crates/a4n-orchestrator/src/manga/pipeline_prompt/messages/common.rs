use a4n_core::error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub(in crate::manga) fn build_manga_step_user_message(
    prompt_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<String, AppError> {
    let payload = json!({
        "task": {
            "prompt_name": prompt_name,
            "instruction": instruction
        },
        "output_contract": output_contract,
        "input": input
    });
    build_json_task_user_message(
        "执行漫画流水线步骤",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}
