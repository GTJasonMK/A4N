use a4n_error::AppError;
use serde_json::Value;

use crate::structured_llm::{ensure_checkpoint_parsed_output_matches, llm_response_preview};

pub(super) fn normalize_scene_generation_output(content: &str) -> Result<String, AppError> {
    let content = content.trim();
    if content.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 未返回可用场景正文".to_string(),
        ));
    }
    if looks_like_structured_or_fenced_scene_response(content) {
        return Err(AppError::InvalidArgument(format!(
            "场景正文 LLM 响应必须是纯文本正文，不能返回 JSON、Markdown 代码块或思考标签；输出预览：{}",
            llm_response_preview(content)
        )));
    }
    Ok(content.to_string())
}

pub(super) fn restored_chapter_scene_checkpoint(
    parsed_output: &Value,
    response_content: &str,
    call_key: &str,
) -> Result<String, AppError> {
    let parsed_output = parsed_output
        .as_str()
        .ok_or_else(|| AppError::InvalidState {
            current: "invalid_checkpoint_field:parsed_output".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        })?;
    let reparsed = normalize_scene_generation_output(response_content)?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &reparsed,
        &format!("章节场景生成 {call_key}"),
    )?;
    Ok(reparsed)
}

fn looks_like_structured_or_fenced_scene_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}

#[test]
fn model_contract_matrix_scene_plain_text_boundary() {
    for (scenario, raw, expected) in [
        ("valid", "她把信放在桌上。", true),
        ("empty", " \n\t", false),
        ("structured", "{\"content\":\"正文\"}", false),
        ("fenced", "```\n正文\n```", false),
        ("thinking", "<think>推理</think>正文", false),
    ] {
        a4n_test_support::model_contract::assert_case(
            "chapter_scene_generation",
            "runtime-text",
            "output",
            scenario,
            serde_json::json!(expected),
            serde_json::json!(normalize_scene_generation_output(raw).is_ok()),
            serde_json::json!({"validator":"normalize_scene_generation_output"}),
        );
    }
}
