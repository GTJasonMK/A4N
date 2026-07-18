use a4n_core::error::AppError;

#[derive(Debug, thiserror::Error)]
pub enum CharacterAttributeLifecycleError {
    #[error(transparent)]
    Persistence(AppError),
    #[error("{0}")]
    Rag(String),
}

impl CharacterAttributeLifecycleError {
    pub(super) fn rag(error: impl std::fmt::Display) -> Self {
        Self::Rag(format!("角色属性事件写入后同步 RAG 失败: {error}"))
    }
}
