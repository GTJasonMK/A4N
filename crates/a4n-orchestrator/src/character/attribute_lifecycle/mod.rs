//! 角色属性生命周期错误类型与纯辅助函数。
//!
//! 数据库写入与 RAG 同步编排已迁入 `context/character.rs`。

use a4n_core::error::AppError;
use a4n_core::models::character::attribute_event;
use a4n_db::character::AppliedCharacterImplicitDecisionRecord;

#[derive(Debug, thiserror::Error)]
pub enum CharacterAttributeLifecycleError {
    #[error(transparent)]
    Persistence(AppError),
    #[error("{0}")]
    Rag(String),
}

impl CharacterAttributeLifecycleError {
    pub(crate) fn rag(error: impl std::fmt::Display) -> Self {
        Self::Rag(format!("角色属性事件写入后同步 RAG 失败: {error}"))
    }
}

/// 从隐式审核决策结果中提取需要同步到 RAG 的属性事件。
pub fn implicit_review_attribute_event(
    applied: &AppliedCharacterImplicitDecisionRecord,
) -> Option<&attribute_event::Model> {
    applied.event.as_ref()
}

#[cfg(test)]
mod tests;
