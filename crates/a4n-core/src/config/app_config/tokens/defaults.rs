use super::super::DEFAULT_LLM_MAX_OUTPUT_TOKENS;
use super::contracts::LlmMaxTokensConfig;

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

pub(super) const fn default_llm_tokens_connection_test() -> usize {
    8
}

pub(super) const fn default_llm_tokens_short() -> usize {
    1024
}

pub(super) const fn default_llm_tokens_standard() -> usize {
    2048
}

pub(super) const fn default_llm_tokens_json_standard() -> usize {
    4096
}

pub(super) const fn default_llm_tokens_long() -> usize {
    8192
}

pub(super) const fn default_llm_tokens_extra_long() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}

pub(super) const fn default_llm_tokens_chapter_generation() -> usize {
    8192
}

pub(super) const fn default_llm_tokens_chapter_analysis() -> usize {
    4096
}

pub(super) const fn default_llm_tokens_outline() -> usize {
    8192
}

pub(super) const fn default_llm_tokens_blueprint() -> usize {
    8192
}

pub(super) const fn default_llm_tokens_character_discovery() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}

pub(super) const fn default_llm_tokens_character_sync() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}

pub(super) const fn default_llm_tokens_media_prompt() -> usize {
    4096
}
