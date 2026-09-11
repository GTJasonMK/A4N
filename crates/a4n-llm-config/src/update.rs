use a4n_error::AppError;

use super::{
    contracts::{CreateLlmConfigInput, NormalizedLlmConfig, UpdateLlmConfigInput},
    create::normalize_create_llm_config_input,
};

pub fn normalize_update_llm_config_input(
    stored_api_key_encrypted: Option<&str>,
    input: UpdateLlmConfigInput<'_>,
) -> Result<NormalizedLlmConfig, AppError> {
    let api_key_encrypted = input
        .api_key_encrypted
        .map(|value| value.map(str::to_string))
        .unwrap_or_else(|| stored_api_key_encrypted.map(str::to_string));
    normalize_create_llm_config_input(CreateLlmConfigInput {
        name: input.name,
        provider: input.provider,
        base_url: input.base_url,
        api_key_encrypted: api_key_encrypted.as_deref(),
        model: input.model,
        max_concurrent: input.max_concurrent,
        timeout_secs: input.timeout_secs,
        max_context_tokens: input.max_context_tokens,
        max_output_tokens: input.max_output_tokens,
        extra_body_json: input.extra_body_json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_preserves_only_an_omitted_api_key() {
        let normalized = normalize_update_llm_config_input(
            Some("stored-key"),
            UpdateLlmConfigInput {
                name: "updated",
                provider: "openai",
                base_url: "https://updated.example/v1",
                api_key_encrypted: None,
                model: "updated-model",
                max_concurrent: 2,
                timeout_secs: 60,
                max_context_tokens: 8192,
                max_output_tokens: 2048,
                extra_body_json: None,
            },
        )
        .unwrap();

        assert_eq!(normalized.name, "updated");
        assert_eq!(normalized.api_key_encrypted.as_deref(), Some("stored-key"));
    }
}
