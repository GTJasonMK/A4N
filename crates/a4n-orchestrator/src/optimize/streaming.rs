use a4n_core::error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;

pub(super) fn ensure_optimize_response_not_truncated(
    finish_reason: Option<&str>,
) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(
            "内容优化模型输出达到当前 max_tokens 上限，返回 JSON 可能被截断；请在设置页调大对应输出档位，或减少本次优化上下文后重试".to_string(),
        ));
    }
    Ok(())
}
