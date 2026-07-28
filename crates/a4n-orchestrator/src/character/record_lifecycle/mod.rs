//! 角色档案删除/时间线回滚生命周期结果类型。
//!
//! 数据库与 RAG 编排已迁入 `context/character.rs`。

mod contracts;

pub use contracts::{
    CharacterProfileRecordDeletionResult, CharacterRecordLifecycleError,
    CharacterTimelineRollbackLifecycleResult,
};

#[cfg(test)]
mod tests;
