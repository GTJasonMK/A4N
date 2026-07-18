use a4n_core::error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;

pub(super) fn ensure_chapter_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(format!(
            "模型生成{label}时达到当前场景输出上限，返回 JSON 被截断；请在设置页调大章节生成 max_tokens，或缩短章节目标/上下文后重试"
        )));
    }
    Ok(())
}
