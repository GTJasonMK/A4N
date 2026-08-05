use a4n_llm::provider::{ChatRequest, UsageInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HttpLlmProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default)]
    pub extra_body_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LlmChatPayload {
    pub provider: HttpLlmProviderConfig,
    pub request: ChatRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LlmProbePayload {
    pub provider: HttpLlmProviderConfig,
    #[serde(default)]
    pub request: Option<ChatRequest>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct LlmProbeOutput {
    pub ok: bool,
    pub response_preview: Option<String>,
    pub usage: Option<UsageInfo>,
    pub finish_reason: Option<String>,
}

fn default_timeout_secs() -> u64 {
    120
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_provider_payload_rejects_unknown_fields() {
        let error = serde_json::from_value::<HttpLlmProviderConfig>(serde_json::json!({
            "base_url": "http://127.0.0.1:1",
            "model": "test",
            "legacy_provider": "openai"
        }))
        .expect_err("unknown LLM provider fields must fail");
        assert!(
            error
                .to_string()
                .contains("unknown field `legacy_provider`")
        );
    }
}
