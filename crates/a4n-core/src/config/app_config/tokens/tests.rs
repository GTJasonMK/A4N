use serde_json::json;

use super::{LlmMaxTokensConfig, llm_token_profiles};
use crate::config::app_config::DEFAULT_LLM_MAX_OUTPUT_TOKENS;

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
fn profile_catalog_preserves_stable_order_and_unknown_profile_contract() {
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
}

#[test]
fn validation_rejects_zero_and_global_max_with_profile_names() {
    let mut config = LlmMaxTokensConfig::default();
    config.connection_test = 0;
    assert_eq!(
        config.validate(),
        Err("connection_test 必须大于 0".to_string())
    );

    let mut config = LlmMaxTokensConfig::default();
    config.media_prompt = 262_145;
    assert_eq!(
        config.validate(),
        Err("media_prompt 不能超过 262144".to_string())
    );
}

#[test]
fn model_output_cap_rejects_oversized_profiles_in_catalog_order() {
    let mut config = LlmMaxTokensConfig::default();
    config.chapter_analysis = 4097;
    config.media_prompt = 4098;

    assert_eq!(
        config.validate_against_model_output_cap(4096),
        Err(
            "场景请求输出预算不能超过当前默认模型最大输出 4096 tokens：long=8192, extra_long=8192, chapter_generation=8192, chapter_analysis=4097, outline=8192, blueprint=8192, character_discovery=8192, character_sync=8192, media_prompt=4098"
                .to_string()
        )
    );
    assert_eq!(
        config.validate_against_model_output_cap(0),
        Err("模型最大输出必须大于 0".to_string())
    );
}

#[test]
fn serde_defaults_missing_profiles_without_changing_explicit_values() {
    let config: LlmMaxTokensConfig =
        serde_json::from_value(json!({"short": 99})).expect("valid token config");
    assert_eq!(config.short, 99);
    assert_eq!(config.connection_test, 8);
    assert_eq!(config.media_prompt, 4096);
}
