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

pub const CHARACTER_DISCOVERY_KIND: &str = "character_discovery";
pub const CHARACTER_DISCOVERY_CALL_KEY: &str = "character_discovery.llm";
pub const CHARACTER_DISCOVERY_PROGRESS_TOTAL: u32 = 100;

pub use a4n_api_contract::character::DiscoverCharacterProfilesInput;
pub use context::character_llm_chapter_context;
pub use contracts::{CharacterDiscoveryProgress, CharacterDiscoveryRequestPreview};
pub use execution::validate_character_discovery_input;
pub use identity::{character_identity_key, profile_character_keys};

#[cfg(test)]
mod tests;
