use a4n_error::AppError;

pub(crate) fn video_llm_step_error(prompt_name: &str, err: AppError) -> AppError {
    match err {
        AppError::Timeout(message) => {
            AppError::Timeout(format!("视频提示词步骤 {prompt_name} 超时：{message}"))
        }
        AppError::Internal(error) => internal_video_error(format!(
            "视频提示词步骤 {prompt_name} 调用 LLM 失败：{error}"
        )),
        AppError::LlmError(message) => AppError::LlmError(format!(
            "视频提示词步骤 {prompt_name} 调用 LLM 失败：{message}"
        )),
        other => other,
    }
}

fn internal_video_error(message: impl Into<String>) -> AppError {
    AppError::Internal(anyhow::anyhow!(message.into()))
}

pub(crate) fn video_db_error(error: a4n_db::video::VideoDbError) -> AppError {
    match error {
        a4n_db::video::VideoDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        a4n_db::video::VideoDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        a4n_db::video::VideoDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::video::VideoDbError::Forbidden(message) => AppError::Forbidden(message),
        a4n_db::video::VideoDbError::Database(error) => AppError::Database(error),
    }
}
