use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::provider::ChatRequest;

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

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LlmProbePayload {
    pub provider: LlmGatewayProviderConfig,
    pub request: ChatRequest,
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
    use super::{LlmGatewayPayload, LlmGatewayProviderConfig, LlmProbePayload};
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
    }
}
