use a4n_core::error::AppError;
use serde_json::Value;

use crate::structured_llm::llm_response_preview;

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
    checkpoint_json: Value,
    call_key: &str,
) -> Result<String, AppError> {
    let parsed_output = required_checkpoint_string(&checkpoint_json, call_key, "parsed_output")?;
    let response_content =
        required_checkpoint_string(&checkpoint_json, call_key, "response_content")?;
    let reparsed = normalize_scene_generation_output(&response_content)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        });
    }
    Ok(reparsed)
}

fn required_checkpoint_string(
    checkpoint_json: &Value,
    call_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("missing_or_invalid_checkpoint_field:{field}"),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        })
}

fn looks_like_structured_or_fenced_scene_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}
