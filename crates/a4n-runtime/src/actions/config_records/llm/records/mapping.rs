use a4n_llm::config::NormalizedLlmConfig;
use a4n_orchestrator::config::{LlmConfigCreateRecordInput, LlmConfigUpdateRecordInput};

pub(super) fn llm_create_record_input(
    normalized: NormalizedLlmConfig,
) -> LlmConfigCreateRecordInput {
    let NormalizedLlmConfig {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        request_options_json,
    } = normalized;
    LlmConfigCreateRecordInput {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        request_options_json,
    }
}

pub(super) fn llm_update_record_input(
    normalized: NormalizedLlmConfig,
) -> LlmConfigUpdateRecordInput {
    let NormalizedLlmConfig {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        request_options_json,
    } = normalized;
    LlmConfigUpdateRecordInput {
        name,
        provider,
        base_url,
        api_key_encrypted,
        model,
        max_concurrent,
        timeout_secs,
        max_context_tokens,
        max_output_tokens,
        request_options_json,
    }
}
