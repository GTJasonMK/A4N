//! 角色一致性检查 application。
//!
//! 统一拥有输入合同、上下文与请求构造、workflow/checkpoint、LLM 调用和输出校验。

mod contracts;
mod execution;
mod request;
mod workflow;

pub(crate) use a4n_character_host_contract::CharacterConflictApplicationPort;
pub(crate) use a4n_character_host_contract::{
    CHARACTER_CONFLICT_KIND, CHARACTER_CONFLICT_PROGRESS_TOTAL,
};

pub use execution::validate_character_conflict_input;

#[cfg(test)]
mod tests;
