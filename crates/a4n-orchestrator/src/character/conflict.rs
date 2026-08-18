//! 角色一致性检查 application。
//!
//! 统一拥有输入合同、上下文与请求构造、workflow/checkpoint、LLM 调用和输出校验。

mod contracts;
mod execution;
mod request;
mod workflow;

pub const CHARACTER_CONFLICT_KIND: &str = "character_conflict_check";
pub const CHARACTER_CONFLICT_PROGRESS_TOTAL: u32 = 4;

pub use contracts::{
    CharacterConflictCheckInput, CharacterConflictProgressReporter, CharacterConflictSummary,
};
pub use execution::validate_character_conflict_input;

#[cfg(test)]
mod tests;
