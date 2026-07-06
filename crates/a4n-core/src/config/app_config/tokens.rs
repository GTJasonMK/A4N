use serde::{Deserialize, Serialize};

use super::DEFAULT_LLM_MAX_OUTPUT_TOKENS;

/// LLM 各场景请求输出预算配置。
///
/// 运行时优先从 system_settings.key = "llm_max_tokens" 读取；这里的默认值同时作为
/// 配置文件默认值和数据库记录缺失时的默认预算。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmMaxTokensConfig {
    #[serde(default = "default_llm_tokens_connection_test")]
    pub connection_test: usize,
    #[serde(default = "default_llm_tokens_short")]
    pub short: usize,
    #[serde(default = "default_llm_tokens_standard")]
    pub standard: usize,
    #[serde(default = "default_llm_tokens_json_standard")]
    pub json_standard: usize,
    #[serde(default = "default_llm_tokens_long")]
    pub long: usize,
    #[serde(default = "default_llm_tokens_extra_long")]
    pub extra_long: usize,
    #[serde(default = "default_llm_tokens_chapter_generation")]
    pub chapter_generation: usize,
    #[serde(default = "default_llm_tokens_chapter_analysis")]
    pub chapter_analysis: usize,
    #[serde(default = "default_llm_tokens_outline")]
    pub outline: usize,
    #[serde(default = "default_llm_tokens_blueprint")]
    pub blueprint: usize,
    #[serde(default = "default_llm_tokens_character_discovery")]
    pub character_discovery: usize,
    #[serde(default = "default_llm_tokens_character_sync")]
    pub character_sync: usize,
    #[serde(default = "default_llm_tokens_media_prompt")]
    pub media_prompt: usize,
}

pub mod llm_token_profiles {
    pub const CONNECTION_TEST: &str = "connection_test";
    pub const SHORT: &str = "short";
    pub const STANDARD: &str = "standard";
    pub const JSON_STANDARD: &str = "json_standard";
    pub const LONG: &str = "long";
    pub const EXTRA_LONG: &str = "extra_long";
    pub const CHAPTER_GENERATION: &str = "chapter_generation";
    pub const CHAPTER_ANALYSIS: &str = "chapter_analysis";
    pub const OUTLINE: &str = "outline";
    pub const BLUEPRINT: &str = "blueprint";
    pub const CHARACTER_DISCOVERY: &str = "character_discovery";
    pub const CHARACTER_SYNC: &str = "character_sync";
    pub const MEDIA_PROMPT: &str = "media_prompt";
}

impl LlmMaxTokensConfig {
    pub fn profile_values(&self) -> [(&'static str, usize); 13] {
        [
            ("connection_test", self.connection_test),
            ("short", self.short),
            ("standard", self.standard),
            ("json_standard", self.json_standard),
            ("long", self.long),
            ("extra_long", self.extra_long),
            ("chapter_generation", self.chapter_generation),
            ("chapter_analysis", self.chapter_analysis),
            ("outline", self.outline),
            ("blueprint", self.blueprint),
            ("character_discovery", self.character_discovery),
            ("character_sync", self.character_sync),
            ("media_prompt", self.media_prompt),
        ]
    }

    pub fn for_profile(&self, profile: &str) -> Option<usize> {
        use llm_token_profiles::*;
        Some(match profile {
            CONNECTION_TEST => self.connection_test,
            SHORT => self.short,
            STANDARD => self.standard,
            JSON_STANDARD => self.json_standard,
            LONG => self.long,
            EXTRA_LONG => self.extra_long,
            CHAPTER_GENERATION => self.chapter_generation,
            CHAPTER_ANALYSIS => self.chapter_analysis,
            OUTLINE => self.outline,
            BLUEPRINT => self.blueprint,
            CHARACTER_DISCOVERY => self.character_discovery,
            CHARACTER_SYNC => self.character_sync,
            MEDIA_PROMPT => self.media_prompt,
            _ => return None,
        })
    }

    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in self.profile_values() {
            if value == 0 {
                return Err(format!("{label} 必须大于 0"));
            }
            if value > 262_144 {
                return Err(format!("{label} 不能超过 262144"));
            }
        }
        Ok(())
    }

    pub fn validate_against_model_output_cap(
        &self,
        model_max_output_tokens: usize,
    ) -> Result<(), String> {
        if model_max_output_tokens == 0 {
            return Err("模型最大输出必须大于 0".to_string());
        }
        let over_limit: Vec<String> = self
            .profile_values()
            .into_iter()
            .filter(|(_, value)| *value > model_max_output_tokens)
            .map(|(label, value)| format!("{label}={value}"))
            .collect();
        if over_limit.is_empty() {
            return Ok(());
        }
        Err(format!(
            "场景请求输出预算不能超过当前默认模型最大输出 {model_max_output_tokens} tokens：{}",
            over_limit.join(", ")
        ))
    }
}

impl Default for LlmMaxTokensConfig {
    fn default() -> Self {
        Self {
            connection_test: default_llm_tokens_connection_test(),
            short: default_llm_tokens_short(),
            standard: default_llm_tokens_standard(),
            json_standard: default_llm_tokens_json_standard(),
            long: default_llm_tokens_long(),
            extra_long: default_llm_tokens_extra_long(),
            chapter_generation: default_llm_tokens_chapter_generation(),
            chapter_analysis: default_llm_tokens_chapter_analysis(),
            outline: default_llm_tokens_outline(),
            blueprint: default_llm_tokens_blueprint(),
            character_discovery: default_llm_tokens_character_discovery(),
            character_sync: default_llm_tokens_character_sync(),
            media_prompt: default_llm_tokens_media_prompt(),
        }
    }
}

const fn default_llm_tokens_connection_test() -> usize {
    8
}
const fn default_llm_tokens_short() -> usize {
    1024
}
const fn default_llm_tokens_standard() -> usize {
    2048
}
const fn default_llm_tokens_json_standard() -> usize {
    4096
}
const fn default_llm_tokens_long() -> usize {
    8192
}
const fn default_llm_tokens_extra_long() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}
const fn default_llm_tokens_chapter_generation() -> usize {
    8192
}
const fn default_llm_tokens_chapter_analysis() -> usize {
    4096
}
const fn default_llm_tokens_outline() -> usize {
    8192
}
const fn default_llm_tokens_blueprint() -> usize {
    8192
}
const fn default_llm_tokens_character_discovery() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}
const fn default_llm_tokens_character_sync() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}
const fn default_llm_tokens_media_prompt() -> usize {
    4096
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_character_sync_tokens_match_large_json_contract() {
        let config = LlmMaxTokensConfig::default();

        assert_eq!(
            config.for_profile(llm_token_profiles::CHARACTER_SYNC),
            Some(DEFAULT_LLM_MAX_OUTPUT_TOKENS)
        );
        assert_eq!(config.character_sync, config.character_discovery);
        assert_eq!(config.extra_long, DEFAULT_LLM_MAX_OUTPUT_TOKENS);
    }

    #[test]
    fn model_output_cap_rejects_oversized_profiles() {
        let mut config = LlmMaxTokensConfig::default();
        config.media_prompt = 4097;
        config.chapter_analysis = 4096;

        let err = config
            .validate_against_model_output_cap(4096)
            .expect_err("profile above model cap must be rejected");

        assert!(err.contains("默认模型最大输出 4096"));
        assert!(err.contains("media_prompt=4097"));
        assert!(!err.contains("chapter_analysis=4096"));
    }
}
