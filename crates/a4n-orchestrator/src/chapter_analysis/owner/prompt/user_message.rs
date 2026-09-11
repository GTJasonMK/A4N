use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub fn build_chapter_analysis_stage_user_message(
    task_name: &str,
    instruction: &str,
    input: Value,
    output_contract: Value,
) -> Result<String, AppError> {
    let payload = json!({
        "task": {
            "name": task_name,
            "instruction": instruction
        },
        "input": input,
        "output_contract": output_contract
    });
    build_json_task_user_message(
        "执行章节结构化分析阶段",
        "只返回 output_contract 要求的严格 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}
