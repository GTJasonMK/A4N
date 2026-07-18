use a4n_core::error::AppError;

use self::gates::looks_like_structured_or_fenced_response;
use self::preview::response_preview;

mod gates;
mod preview;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::chapter_generation::service) struct GeneratedChapterPayload {
    pub(in crate::chapter_generation::service) title: String,
    pub(in crate::chapter_generation::service) summary: String,
    pub(in crate::chapter_generation::service) content: String,
}

#[derive(Debug, Clone)]
pub(in crate::chapter_generation::service) struct GeneratedChapterMetadata {
    pub(in crate::chapter_generation::service) title: String,
    pub(in crate::chapter_generation::service) summary: String,
}

pub(in crate::chapter_generation::service) fn normalize_generated_chapter_response(
    raw: &str,
    metadata: &GeneratedChapterMetadata,
) -> Result<GeneratedChapterPayload, AppError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 未返回可用章节正文，未写入正文".to_string(),
        ));
    }
    if looks_like_structured_or_fenced_response(trimmed) {
        return Err(AppError::InvalidArgument(format!(
            "章节正文 LLM 响应必须是纯文本正文，不能返回 JSON、Markdown 代码块或思考标签；输出预览：{}",
            response_preview(trimmed)
        )));
    }

    Ok(GeneratedChapterPayload {
        title: metadata.title.clone(),
        summary: metadata.summary.clone(),
        content: trimmed.to_string(),
    })
}
