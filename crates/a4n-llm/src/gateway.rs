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
pub struct LlmProbeOutput {
    #[serde(deserialize_with = "deserialize_required_option")]
    pub usage: Option<UsageInfo>,
    #[serde(deserialize_with = "deserialize_required_option")]
    pub finish_reason: Option<String>,
}

impl LlmProbeOutput {
    pub fn from_chat_response(response: ChatResponse) -> Result<Self, AppError> {
        if response.message.content.trim().is_empty() {
            return Err(AppError::LlmError("LLM 连接探测未收到有效响应".to_string()));
        }
        Ok(Self {
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
    use super::{LlmGatewayPayload, LlmGatewayProviderConfig, LlmProbeOutput};
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
            serde_json::from_value::<LlmProbeOutput>(json!({
                "finish_reason": "stop"
            }))
            .is_err()
        );
    }
}
