use a4n_core::error::AppError;

pub(super) fn ensure_output_tokens_below_context(
    max_output_tokens: i32,
    max_context_tokens: i32,
) -> Result<(), AppError> {
    if max_output_tokens >= max_context_tokens {
        return Err(AppError::InvalidArgument(
            "LLM配置 max_output_tokens 必须小于 max_context_tokens".into(),
        ));
    }
    Ok(())
}
