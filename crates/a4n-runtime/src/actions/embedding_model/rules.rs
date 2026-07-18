use super::contracts::{EmbeddingModelExecution, LocalEmbeddingModelSummary};
use a4n_core::error::AppError;

pub(super) fn embedding_model_execution(
    profile: crate::state::RuntimeProfile,
) -> EmbeddingModelExecution {
    if profile.uses_sidecars() {
        EmbeddingModelExecution::Sidecar
    } else {
        EmbeddingModelExecution::HostLocal
    }
}

pub(super) fn downloaded_model_summary(
    models: Vec<LocalEmbeddingModelSummary>,
    model: &str,
) -> Result<LocalEmbeddingModelSummary, AppError> {
    models
        .into_iter()
        .find(|item| {
            item.name.eq_ignore_ascii_case(model) || item.model_code.eq_ignore_ascii_case(model)
        })
        .ok_or_else(|| AppError::ConfigError(format!("下载完成但未能定位模型状态: {model}")))
}

pub(super) fn required_text(value: &str, label: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label}不能为空")))
    } else {
        Ok(value.to_string())
    }
}
