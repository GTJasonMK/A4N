use a4n_llm::provider::{ChatRequest, UsageInfo};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(in crate::dynamic_llm) struct LlmGatewayProviderConfig {
    pub(in crate::dynamic_llm) base_url: String,
    pub(in crate::dynamic_llm) model: String,
    pub(in crate::dynamic_llm) api_key: Option<String>,
    pub(in crate::dynamic_llm) timeout_secs: u64,
    pub(in crate::dynamic_llm) request_options_json: Option<Value>,
}

#[derive(Debug, Serialize)]
pub(in crate::dynamic_llm) struct LlmChatPayload {
    pub(in crate::dynamic_llm) provider: LlmGatewayProviderConfig,
    pub(in crate::dynamic_llm) request: ChatRequest,
}

#[derive(Debug, Serialize)]
pub(in crate::dynamic_llm) struct LlmProbePayload {
    pub(in crate::dynamic_llm) provider: LlmGatewayProviderConfig,
    pub(in crate::dynamic_llm) request: Option<ChatRequest>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::dynamic_llm) struct LlmProbeOutput {
    pub(in crate::dynamic_llm) ok: bool,
    pub(in crate::dynamic_llm) response_preview: Option<String>,
    pub(in crate::dynamic_llm) usage: Option<UsageInfo>,
    pub(in crate::dynamic_llm) finish_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::LlmProbeOutput;

    #[test]
    fn llm_probe_output_rejects_unknown_fields() {
        let output = serde_json::from_value::<LlmProbeOutput>(serde_json::json!({
            "ok": true,
            "response_preview": "pong",
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": 2,
                "total_tokens": 3,
                "prompt_cache_hit_tokens": null,
                "prompt_cache_miss_tokens": null
            },
            "finish_reason": "stop"
        }))
        .expect("gateway probe fields must deserialize");
        assert_eq!(output.usage.map(|usage| usage.total_tokens), Some(3));
        assert_eq!(output.finish_reason.as_deref(), Some("stop"));

        let error = serde_json::from_value::<LlmProbeOutput>(serde_json::json!({
            "ok": true,
            "response_preview": "pong",
            "usage": null,
            "finish_reason": null,
            "legacy_result": true
        }))
        .expect_err("LLM probe output must reject unknown fields");
        assert!(error.to_string().contains("legacy_result"));
    }
}
