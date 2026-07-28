//! 角色发现 application。
//!
//! 统一拥有上下文、请求、streaming LLM、checkpoint、输出校验、档案落库和 evidence 绑定。

mod context;
mod contracts;
mod execution;
mod identity;
mod output;
mod persistence;
mod request;

pub use context::character_llm_chapter_context;
pub use contracts::{
    CharacterDiscoveryInput, CharacterDiscoveryProgress, CharacterDiscoveryRequestPreview,
    CharacterDiscoveryTaskScope,
};
pub use identity::{character_identity_key, profile_character_keys};

#[cfg(test)]
mod tests;
