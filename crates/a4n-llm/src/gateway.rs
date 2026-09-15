use serde::{Deserialize, Serialize};

use crate::provider::ChatRequest;

pub use a4n_llm_sidecar_contract::LlmGatewayProviderConfig;

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

#[cfg(test)]
mod tests {
    use super::{LlmGatewayPayload, LlmProbePayload};
    use serde_json::json;

    #[test]
    fn gateway_contract_rejects_missing_wire_fields() {
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
