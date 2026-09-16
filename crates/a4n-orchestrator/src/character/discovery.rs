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

pub use a4n_character_host_contract::{
    CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND, CHARACTER_DISCOVERY_PROGRESS_TOTAL,
};

pub use context::character_llm_chapter_context;
pub use execution::validate_character_discovery_input;
pub use identity::{character_identity_key, profile_character_keys};

#[cfg(test)]
mod tests;
