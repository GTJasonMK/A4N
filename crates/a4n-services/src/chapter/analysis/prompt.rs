use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter;
use a4n_engine::prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use super::normalization::analysis_schema_error;

pub(super) fn build_chapter_analysis_stage_user_message(
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

pub(super) fn required_chapter_title(ch: &chapter::Model) -> Result<&str, AppError> {
    ch.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            analysis_schema_error(format!(
                "第{}章缺少章节标题，不能执行章节分析",
                ch.chapter_number
            ))
        })
}
