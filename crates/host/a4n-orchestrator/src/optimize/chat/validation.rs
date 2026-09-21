use a4n_db_models::models::ai::optimization_session;
use a4n_error::AppError;

pub(in crate::optimize) fn required_chat_user_text(message: &str) -> Result<&str, AppError> {
    let user_text = message.trim();
    if user_text.is_empty() {
        return Err(AppError::InvalidArgument("对话消息不能为空".to_string()));
    }
    Ok(user_text)
}

pub(in crate::optimize) fn ensure_chat_session_accepts_message(
    session: &optimization_session::Model,
    operation: &str,
) -> Result<(), AppError> {
    if matches!(
        session.status.as_str(),
        "analyzing" | "fixing" | "cancelled"
    ) {
        return Err(AppError::InvalidState {
            current: session.status.clone(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::required_chat_user_text;

    #[test]
    fn chat_user_text_is_trimmed() {
        assert_eq!(required_chat_user_text("  继续优化  ").unwrap(), "继续优化");
    }

    #[test]
    fn chat_user_text_rejects_whitespace_only_input() {
        let error = required_chat_user_text(" \n\t ").unwrap_err();
        assert_eq!(error.to_string(), "参数无效：对话消息不能为空");
    }
}
