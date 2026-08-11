use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use a4n_core::error::AppError;

use crate::provider::{ChatRequest, ChatResponse, UsageInfo};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlmGatewayProviderConfig {
    pub provider: String,
    pub base_url: String,
    pub model: String,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub api_key: Option<String>,
    pub timeout_secs: u64,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub extra_body_json: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlmGatewayPayload {
    pub provider: LlmGatewayProviderConfig,
    pub request: ChatRequest,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlmProbePayload {
    pub provider: LlmGatewayProviderConfig,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub request: Option<ChatRequest>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlmProbeOutput {
    pub ok: bool,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub response_preview: Option<String>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub usage: Option<UsageInfo>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub finish_reason: Option<String>,
}

impl LlmProbeOutput {
    pub fn from_chat_response(response: ChatResponse) -> Result<Self, AppError> {
        let content = response.message.content.trim();
        if content.is_empty() {
            return Err(AppError::LlmError("LLM 连接探测未收到有效响应".to_string()));
        }
        Ok(Self {
            ok: true,
            response_preview: Some(content.chars().take(200).collect()),
            usage: response.usage,
            finish_reason: response.finish_reason,
        })
    }
}

fn deserialize_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::{LlmGatewayPayload, LlmGatewayProviderConfig, LlmProbeOutput, LlmProbePayload};
    use serde_json::json;

    #[test]
    fn gateway_contract_rejects_missing_wire_fields() {
        assert!(
            serde_json::from_value::<LlmGatewayProviderConfig>(json!({
                "provider": "openai",
                "base_url": "https://example.invalid/v1",
                "model": "test",
                "timeout_secs": 30,
                "extra_body_json": null
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<LlmGatewayPayload>(json!({
                "provider": {
                    "provider": "openai",
                    "base_url": "https://example.invalid/v1",
                    "model": "test",
                    "api_key": null,
                    "timeout_secs": 30,
                    "extra_body_json": null
                }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<LlmProbePayload>(json!({
                "provider": {
                    "provider": "openai",
                    "base_url": "https://example.invalid/v1",
                    "model": "test",
                    "api_key": null,
                    "timeout_secs": 30,
                    "extra_body_json": null
                }
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<LlmProbeOutput>(json!({
                "ok": true,
                "response_preview": "pong",
                "finish_reason": "stop"
            }))
            .is_err()
        );
    }

    #[test]
    fn probe_output_uses_the_shared_runtime_contract() {
        let output = serde_json::from_value::<LlmProbeOutput>(json!({
            "ok": true,
            "response_preview": "pong",
            "usage": null,
            "finish_reason": "stop"
        }))
        .expect("probe output should deserialize");

        assert!(output.ok);
        assert_eq!(output.response_preview.as_deref(), Some("pong"));
    }
}
