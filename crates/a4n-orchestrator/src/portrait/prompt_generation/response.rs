use a4n_core::error::AppError;

pub fn normalize_portrait_prompt_response(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "人物三视图提示词 LLM 响应为空".to_string(),
        ));
    }
    if looks_like_structured_or_fenced_response(trimmed) {
        return Err(AppError::InvalidArgument(format!(
            "人物三视图提示词必须是纯 prompt 文本，不能返回 JSON、Markdown 代码块或思考标签；输出预览：{}",
            portrait_response_preview(trimmed)
        )));
    }
    Ok(trimmed.to_string())
}

fn looks_like_structured_or_fenced_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('#')
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}

fn portrait_response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    trimmed.chars().take(180).collect()
}
