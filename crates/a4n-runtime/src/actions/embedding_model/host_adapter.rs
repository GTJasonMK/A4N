//! Host 文本输入适配。

use a4n_core::error::AppError;

/// Host 文本必填：orch 返回 &str，此处转 owned 供后续调用。
pub(super) fn required_text(value: &str, label: &str) -> Result<String, AppError> {
    a4n_orchestrator::project::required_text(value, label)
        .map(str::to_string)
        .map_err(AppError::InvalidArgument)
}
