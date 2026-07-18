use a4n_core::error::AppError;

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
            scene_response_preview(content)
        )));
    }
    Ok(content.to_string())
}

fn looks_like_structured_or_fenced_scene_response(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with("```")
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with("<think>")
}

fn scene_response_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "<empty>".to_string();
    }
    let prefix = trimmed.chars().take(180).collect::<String>();
    if trimmed.chars().count() <= 260 {
        return prefix;
    }
    let suffix = trimmed
        .chars()
        .rev()
        .take(80)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix} … {suffix}")
}
