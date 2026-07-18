use crate::request::build_json_task_user_message;
use a4n_core::error::AppError;
use serde_json::json;

#[cfg(test)]
mod tests;

pub(super) fn build_chapter_generation_user_message(
    chapter_outline: &str,
    context_before: Option<&str>,
) -> Result<String, AppError> {
    required_generation_outline_text(chapter_outline)?;
    let payload = json!({
        "task": {
            "name": "chapter_generation",
            "instruction": "根据章节大纲、前文场景状态和参考上下文生成完整章节正文"
        },
        "context_refs": {
            "chapter_outline": "稳定上下文包 writing_chapter_outline",
            "context_before": if context_before.map(str::trim).filter(|text| !text.is_empty()).is_some() {
                json!("稳定上下文包 writing_context_before")
            } else {
                serde_json::Value::Null
            }
        },
        "output_contract": {
            "format": "plain_text",
            "content": "完整章节正文，必须满足 chapter_outline 中的正文字数要求；纯文本，包含自然段落换行，不含 Markdown/HTML/标题标记",
            "start_rule": "直接从正文第一自然段开始，不要先输出任何标签、对象包装、字段名、说明或摘要",
            "wrapper_rule": "不要输出结构化对象、数组、键值对或代码块；章节元数据由后端维护"
        }
    });
    build_json_task_user_message(
        "生成章节正文",
        "只返回章节正文纯文本，不要输出 JSON、Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}

pub(super) fn required_generation_outline_text(chapter_outline: &str) -> Result<&str, AppError> {
    let outline = chapter_outline.trim();
    if outline.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节生成缺少章节大纲，未调用 LLM".to_string(),
        ));
    }
    Ok(outline)
}

pub(super) fn normalize_context_before_section(context_before: Option<&str>) -> Option<String> {
    match context_before
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(text) if text.starts_with('#') => Some(text.to_string()),
        Some(text) => Some(format!("## 场景状态\n{text}")),
        None => None,
    }
}
