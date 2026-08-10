use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DEFAULT_LLM_MAX_CONTEXT_TOKENS: usize = 128_000;
pub const DEFAULT_LLM_MAX_OUTPUT_TOKENS: usize = 8_192;
pub const LLM_MAX_TOKEN_LIMIT: usize = 262_144;

pub const LLM_RESERVED_REQUEST_OPTION_KEYS: &[&str] = &[
    "model",
    "messages",
    "stream",
    "temperature",
    "max_tokens",
    "response_format",
    "prompt_cache_key",
    "tools",
    "tool_choice",
    "stop",
    "stop_sequences",
    "system",
    "anthropic_version",
    "cache_control",
];

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

/// LLM 各场景请求输出预算配置。
///
/// 数据库 system setting、本机 runtime setting 和 AppConfig 共用这一份合同。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct LlmMaxTokensConfig {
    pub connection_test: usize,
    pub short: usize,
    pub standard: usize,
    pub json_standard: usize,
    pub long: usize,
    pub extra_long: usize,
    pub chapter_generation: usize,
    pub chapter_analysis: usize,
    pub outline: usize,
    pub blueprint: usize,
    pub character_discovery: usize,
    pub character_sync: usize,
    pub media_prompt: usize,
}

impl Default for LlmMaxTokensConfig {
    fn default() -> Self {
        Self {
            connection_test: 8,
            short: 1024,
            standard: 2048,
            json_standard: 4096,
            long: 8192,
            extra_long: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            chapter_generation: 8192,
            chapter_analysis: 4096,
            outline: 8192,
            blueprint: 8192,
            character_discovery: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            character_sync: DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            media_prompt: 4096,
        }
    }
}

impl LlmMaxTokensConfig {
    pub fn profile_values(&self) -> [(&'static str, usize); 13] {
        use llm_token_profiles::*;

        [
            (CONNECTION_TEST, self.connection_test),
            (SHORT, self.short),
            (STANDARD, self.standard),
            (JSON_STANDARD, self.json_standard),
            (LONG, self.long),
            (EXTRA_LONG, self.extra_long),
            (CHAPTER_GENERATION, self.chapter_generation),
            (CHAPTER_ANALYSIS, self.chapter_analysis),
            (OUTLINE, self.outline),
            (BLUEPRINT, self.blueprint),
            (CHARACTER_DISCOVERY, self.character_discovery),
            (CHARACTER_SYNC, self.character_sync),
            (MEDIA_PROMPT, self.media_prompt),
        ]
    }

    pub fn for_profile(&self, profile: &str) -> Option<usize> {
        self.profile_values()
            .into_iter()
            .find_map(|(candidate, value)| (candidate == profile).then_some(value))
    }

    pub fn validate(&self) -> Result<(), String> {
        for (label, value) in self.profile_values() {
            if value == 0 {
                return Err(format!("{label} 必须大于 0"));
            }
            if value > LLM_MAX_TOKEN_LIMIT {
                return Err(format!("{label} 不能超过 {LLM_MAX_TOKEN_LIMIT}"));
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        DEFAULT_LLM_MAX_OUTPUT_TOKENS, LLM_MAX_TOKEN_LIMIT, LlmMaxTokensConfig, llm_token_profiles,
    };

    #[test]
    fn token_contract_has_one_default_and_strict_shape() {
        let config = LlmMaxTokensConfig::default();
        assert_eq!(
            config.for_profile(llm_token_profiles::CHARACTER_SYNC),
            Some(DEFAULT_LLM_MAX_OUTPUT_TOKENS)
        );
        assert!(
            serde_json::from_value::<LlmMaxTokensConfig>(json!({
                "short": 99
            }))
            .is_err()
        );
    }

    #[test]
    fn profile_catalog_and_validation_are_canonical() {
        let config = LlmMaxTokensConfig::default();
        assert_eq!(
            config
                .profile_values()
                .map(|(profile, _)| profile)
                .as_slice(),
            [
                "connection_test",
                "short",
                "standard",
                "json_standard",
                "long",
                "extra_long",
                "chapter_generation",
                "chapter_analysis",
                "outline",
                "blueprint",
                "character_discovery",
                "character_sync",
                "media_prompt",
            ]
        );
        assert_eq!(config.for_profile("unknown"), None);

        let invalid = LlmMaxTokensConfig {
            connection_test: 0,
            ..config.clone()
        };
        assert_eq!(
            invalid.validate(),
            Err("connection_test 必须大于 0".to_string())
        );

        let invalid = LlmMaxTokensConfig {
            media_prompt: LLM_MAX_TOKEN_LIMIT + 1,
            ..config
        };
        assert_eq!(
            invalid.validate(),
            Err("media_prompt 不能超过 262144".to_string())
        );
    }

    #[test]
    fn model_output_cap_rejects_oversized_profiles_in_catalog_order() {
        let config = LlmMaxTokensConfig {
            chapter_analysis: 4097,
            media_prompt: 4098,
            ..Default::default()
        };
        assert_eq!(
            config.validate_against_model_output_cap(4096),
            Err(
                "场景请求输出预算不能超过当前默认模型最大输出 4096 tokens：long=8192, extra_long=8192, chapter_generation=8192, chapter_analysis=4097, outline=8192, blueprint=8192, character_discovery=8192, character_sync=8192, media_prompt=4098"
                    .to_string()
            )
        );
    }
}
