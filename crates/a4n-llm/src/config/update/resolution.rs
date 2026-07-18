use a4n_core::models::ai::llm_config;
use serde_json::Value;

use super::super::contracts::UpdateLlmConfigInput;

pub(super) struct ResolvedLlmConfigUpdate {
    pub(super) name: String,
    pub(super) provider: String,
    pub(super) base_url: String,
    pub(super) api_key_encrypted: Option<String>,
    pub(super) model: String,
    pub(super) max_concurrent: i32,
    pub(super) timeout_secs: i32,
    pub(super) max_context_tokens: i32,
    pub(super) max_output_tokens: i32,
    pub(super) request_options_json: Option<Value>,
}

pub(super) fn resolve_update_llm_config_input(
    cfg: &llm_config::Model,
    input: UpdateLlmConfigInput<'_>,
) -> ResolvedLlmConfigUpdate {
    ResolvedLlmConfigUpdate {
        name: input
            .name
            .map(str::to_string)
            .unwrap_or_else(|| cfg.name.clone()),
        provider: input
            .provider
            .map(str::to_string)
            .unwrap_or_else(|| cfg.provider.clone()),
        base_url: input
            .base_url
            .map(str::to_string)
            .unwrap_or_else(|| cfg.base_url.clone()),
        api_key_encrypted: input
            .api_key_encrypted
            .map(|value| value.map(str::to_string))
            .unwrap_or_else(|| cfg.api_key_encrypted.clone()),
        model: input
            .model
            .map(str::to_string)
            .unwrap_or_else(|| cfg.model.clone()),
        max_concurrent: input.max_concurrent.unwrap_or(cfg.max_concurrent),
        timeout_secs: input.timeout_secs.unwrap_or(cfg.timeout_secs),
        max_context_tokens: input.max_context_tokens.unwrap_or(cfg.max_context_tokens),
        max_output_tokens: input.max_output_tokens.unwrap_or(cfg.max_output_tokens),
        request_options_json: input
            .request_options_json
            .unwrap_or_else(|| cfg.request_options_json.clone()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn stored_config() -> llm_config::Model {
        llm_config::Model {
            id: 1,
            name: "stored".into(),
            provider: "openai".into(),
            base_url: "https://stored.example".into(),
            api_key_encrypted: Some("stored-key".into()),
            model: "stored-model".into(),
            max_concurrent: 2,
            timeout_secs: 30,
            max_context_tokens: 4096,
            max_output_tokens: 1024,
            request_options_json: Some(json!({"stored": true})),
            is_default: false,
            created_at: Default::default(),
            updated_at: Default::default(),
        }
    }

    #[test]
    fn omitted_fields_use_stored_values() {
        let cfg = stored_config();
        let resolved = resolve_update_llm_config_input(&cfg, UpdateLlmConfigInput::default());

        assert_eq!(resolved.name, cfg.name);
        assert_eq!(resolved.provider, cfg.provider);
        assert_eq!(resolved.base_url, cfg.base_url);
        assert_eq!(resolved.api_key_encrypted, cfg.api_key_encrypted);
        assert_eq!(resolved.model, cfg.model);
        assert_eq!(resolved.max_concurrent, cfg.max_concurrent);
        assert_eq!(resolved.timeout_secs, cfg.timeout_secs);
        assert_eq!(resolved.max_context_tokens, cfg.max_context_tokens);
        assert_eq!(resolved.max_output_tokens, cfg.max_output_tokens);
        assert_eq!(resolved.request_options_json, cfg.request_options_json);
    }

    #[test]
    fn provided_fields_override_and_nullable_fields_can_be_cleared() {
        let cfg = stored_config();
        let input = UpdateLlmConfigInput {
            name: Some("updated"),
            provider: Some("anthropic"),
            base_url: Some("https://updated.example"),
            api_key_encrypted: Some(None),
            model: Some("updated-model"),
            max_concurrent: Some(4),
            timeout_secs: Some(60),
            max_context_tokens: Some(8192),
            max_output_tokens: Some(2048),
            request_options_json: Some(None),
        };

        let resolved = resolve_update_llm_config_input(&cfg, input);

        assert_eq!(resolved.name, "updated");
        assert_eq!(resolved.provider, "anthropic");
        assert_eq!(resolved.base_url, "https://updated.example");
        assert_eq!(resolved.api_key_encrypted, None);
        assert_eq!(resolved.model, "updated-model");
        assert_eq!(resolved.max_concurrent, 4);
        assert_eq!(resolved.timeout_secs, 60);
        assert_eq!(resolved.max_context_tokens, 8192);
        assert_eq!(resolved.max_output_tokens, 2048);
        assert_eq!(resolved.request_options_json, None);
    }
}
