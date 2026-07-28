//! 角色一致性检查 application。
//!
//! 统一拥有输入合同、上下文与请求构造、workflow/checkpoint、LLM 调用和输出校验。

mod contracts;
mod execution;
mod request;
mod workflow;

pub use contracts::{
    CharacterConflictCheckInput, CharacterConflictProgressReporter,
    CharacterConflictRequestPreview, CharacterConflictTaskScope,
};

#[cfg(test)]
mod tests;
